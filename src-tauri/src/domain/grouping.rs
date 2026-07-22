//! Group job summaries into batch rows + singles (Phoenix Jobs.Grouping parity).

use crate::domain::models::JobSummary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum JobListRow {
    #[serde(rename = "single")]
    Single { job: JobSummary },
    /// Field-level camelCase is required: enum `rename_all` does not rename struct-variant fields.
    #[serde(rename = "batch", rename_all = "camelCase")]
    Batch {
        batch_id: i64,
        jobs: Vec<JobSummary>,
        created_at: String,
        status: String,
    },
}

/// Collapse a flat job list into UI rows: one row per batch, one per unbatched job.
/// Newest activity first (by created_at desc).
pub fn build_job_list_rows(jobs: &[JobSummary]) -> Vec<JobListRow> {
    use std::collections::HashMap;

    let mut batches: HashMap<i64, Vec<JobSummary>> = HashMap::new();
    let mut singles: Vec<JobSummary> = Vec::new();

    for job in jobs {
        match job.batch_id {
            Some(batch_id) => {
                batches.entry(batch_id).or_default().push(job.clone());
            }
            None => singles.push(job.clone()),
        }
    }

    let mut batch_rows: Vec<JobListRow> = batches
        .into_iter()
        .map(|(batch_id, mut batch_jobs)| {
            batch_jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            let created_at = batch_jobs
                .first()
                .map(|j| j.created_at.clone())
                .unwrap_or_default();
            let status = summarize_batch_status(&batch_jobs);
            JobListRow::Batch {
                batch_id,
                jobs: batch_jobs,
                created_at,
                status,
            }
        })
        .collect();

    let mut single_rows: Vec<JobListRow> = singles
        .into_iter()
        .map(|job| JobListRow::Single { job })
        .collect();

    let mut rows = Vec::with_capacity(batch_rows.len() + single_rows.len());
    rows.append(&mut batch_rows);
    rows.append(&mut single_rows);

    rows.sort_by(|a, b| {
        let ta = row_created_at(a);
        let tb = row_created_at(b);
        tb.cmp(ta)
    });
    rows
}

fn row_created_at(row: &JobListRow) -> &str {
    match row {
        JobListRow::Single { job } => &job.created_at,
        JobListRow::Batch { created_at, .. } => created_at,
    }
}

/// Batch status: failed > processing > queued > completed (Phoenix summarize_batch_status).
pub fn summarize_batch_status(jobs: &[JobSummary]) -> String {
    if jobs.iter().any(|j| j.status == "failed") {
        return "failed".into();
    }
    if jobs.iter().any(|j| j.status == "processing") {
        return "processing".into();
    }
    if jobs.iter().any(|j| j.status == "queued") {
        return "queued".into();
    }
    "completed".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: i64, batch_id: Option<i64>, status: &str, created_at: &str) -> JobSummary {
        JobSummary {
            id,
            status: status.into(),
            provider_key: "assemblyai".into(),
            batch_id,
            upload_id: id,
            original_filename: format!("f{id}.wav"),
            error_message: None,
            created_at: created_at.into(),
            retryable: status == "failed",
        }
    }

    #[test]
    fn singles_only_newest_first() {
        let jobs = vec![
            job(1, None, "completed", "2026-01-01T10:00:00Z"),
            job(2, None, "queued", "2026-01-02T10:00:00Z"),
        ];
        let rows = build_job_list_rows(&jobs);
        assert_eq!(rows.len(), 2);
        match &rows[0] {
            JobListRow::Single { job } => assert_eq!(job.id, 2),
            other => panic!("expected single, got {other:?}"),
        }
        match &rows[1] {
            JobListRow::Single { job } => assert_eq!(job.id, 1),
            other => panic!("expected single, got {other:?}"),
        }
    }

    #[test]
    fn batches_collapse_into_one_row() {
        let jobs = vec![
            job(1, Some(9), "completed", "2026-01-01T12:00:00Z"),
            job(2, Some(9), "queued", "2026-01-01T11:00:00Z"),
            job(3, None, "failed", "2026-01-03T10:00:00Z"),
        ];
        let rows = build_job_list_rows(&jobs);
        assert_eq!(rows.len(), 2);
        // newest is the single failed job
        match &rows[0] {
            JobListRow::Single { job } => assert_eq!(job.id, 3),
            other => panic!("expected single first, got {other:?}"),
        }
        match &rows[1] {
            JobListRow::Batch {
                batch_id,
                jobs: batch_jobs,
                status,
                ..
            } => {
                assert_eq!(*batch_id, 9);
                assert_eq!(batch_jobs.len(), 2);
                assert_eq!(status, "queued"); // any queued beats completed
            }
            other => panic!("expected batch, got {other:?}"),
        }
    }

    #[test]
    fn summarize_batch_status_priority() {
        assert_eq!(
            summarize_batch_status(&[
                job(1, Some(1), "completed", "t"),
                job(2, Some(1), "failed", "t"),
            ]),
            "failed"
        );
        assert_eq!(
            summarize_batch_status(&[
                job(1, Some(1), "queued", "t"),
                job(2, Some(1), "processing", "t"),
            ]),
            "processing"
        );
        assert_eq!(
            summarize_batch_status(&[
                job(1, Some(1), "completed", "t"),
                job(2, Some(1), "queued", "t"),
            ]),
            "queued"
        );
        assert_eq!(
            summarize_batch_status(&[
                job(1, Some(1), "completed", "t"),
                job(2, Some(1), "completed", "t"),
            ]),
            "completed"
        );
    }

    /// Frontend reads row.batchId — serde must emit camelCase for batch fields
    /// (enum container rename_all does NOT rename struct-variant fields).
    #[test]
    fn batch_row_json_uses_camel_case_for_view_navigation() {
        let rows = build_job_list_rows(&[
            job(1, Some(42), "queued", "2026-01-01T10:00:00Z"),
            job(2, Some(42), "queued", "2026-01-01T11:00:00Z"),
        ]);
        assert_eq!(rows.len(), 1);
        let json = serde_json::to_value(&rows[0]).expect("serialize");
        assert_eq!(json["kind"], "batch", "kind tag for UI branch");
        assert!(
            json.get("batchId").is_some(),
            "must expose batchId (got keys: {:?}) so JobsPage onOpenBatch works",
            json.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
        assert_eq!(json["batchId"], 42);
        assert!(json.get("createdAt").is_some(), "must expose createdAt");
        assert!(json["jobs"].is_array());
        assert_eq!(json["jobs"].as_array().unwrap().len(), 2);
        // UI must not receive snake_case only
        assert!(json.get("batch_id").is_none());
    }
}
