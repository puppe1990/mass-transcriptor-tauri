export type Locale = "en" | "pt-BR";

const STORAGE_KEY = "mt-locale";

const en = {
  sidebar: {
    ariaLabel: "Workspace sidebar",
    uploads: "Uploads",
    jobs: "Jobs",
    settings: "Settings",
    language: "Language",
    onThisMachine: "On this machine · no account",
  },
  language: {
    en: "English",
    "pt-BR": "Português (Brasil)",
  },
  theme: {
    light: "Light",
    dark: "Dark",
    switchToLight: "Switch to light mode",
    switchToDark: "Switch to dark mode",
  },
  upload: {
    eyebrow: "Stage",
    title: "Bring in the tape",
    subtitle:
      "Drop audio or video. Each file becomes a job with the default engine from Settings.",
    viewJobs: "View jobs",
    queuedOne: "1 job queued for transcription.",
    queuedMany: "{{count}} jobs queued for transcription.",
    openGroup: "Open upload group",
    jobLabel: "Job #{{id}}",
    dropActive: "Release to queue",
    dropIdle: "Drop files on this deck",
    formats: "MP3, WAV, OGG, M4A, FLAC or MP4, MOV, WebM, MKV · up to 20 files",
    browse: "Browse files",
    creating: "Creating jobs…",
    cannotUpload: "Cannot upload this file",
    unsupportedType:
      "This file type is not supported. Use common audio formats or MP4, MOV, WebM, and MKV video.",
    dropFailed:
      "Drop failed. Use Browse files, or drop files while the app window is focused.",
  },
  jobs: {
    eyebrow: "Queue",
    title: "Jobs",
    subtitle: "Watch runs, open finished transcripts, retry the ones that missed.",
    newUpload: "New upload",
    total: "Total",
    queued: "Queued",
    processing: "Processing",
    completed: "Completed",
    failed: "Failed",
    emptyTitle: "The bench is empty",
    emptyText: "Drop a file on Stage to cut the first transcript.",
    audios: "{{count}} audios",
  },
  job: {
    loading: "Loading…",
    back: "Back to jobs",
    notFound: "Job not found",
    batch: "Batch #{{id}}",
    provider: "Provider",
    status: "Status",
    output: "Output",
    file: "File",
    retry: "Retry job",
    retrying: "Retrying...",
    cancel: "Cancel",
    cancelling: "Cancelling…",
    downloadMd: "Download markdown",
    empty: "No transcript yet.",
    manuscript: "Manuscript",
    copy: "Copy Text",
    copied: "Copied",
    copyFailed: "Could not copy text.",
  },
  batch: {
    title: "Reel · {{count}} files",
    subtitle: "Switch takes below. Each file is its own cut.",
    downloadAll: "Download all",
    preparing: "Preparing…",
    tabs: "Batch files",
    openFull: "Open full page",
    notFound: "Batch not found. Open it again from Jobs.",
    viewJobs: "View jobs",
  },
  settings: {
    eyebrow: "Booth",
    title: "Settings",
    lede: "Pick the engine, keep keys on this machine, download Whisper weights only when you ask.",
    workspace: "Workspace",
    workspaceHint: "Local desktop app · no multi-user login",
    assemblyNote: "AssemblyAI uses the API key stored in local SQLite settings.",
    whisperNote:
      "Whisper is local and does not use that key. Models stay on disk until you delete them.",
    provider: "Provider",
    defaultProvider: "Default provider",
    transcriptionLanguage: "Transcription language",
    langAuto: "Auto detect",
    langPt: "Portuguese",
    langEn: "English",
    langEs: "Spanish",
    models: "Local Whisper models",
    modelsLede:
      "Download only the sizes you want. Nothing is fetched until you click Download. You can select and save a model only after it is on disk.",
    modelMissing: "Selected model `{{id}}` is not downloaded yet.",
    cannotSelect: "Download this model before you can select it.",
    cannotSave:
      "Download a Whisper model and select it before saving Whisper as the default provider.",
    downloadingPct: "Downloading {{pct}}%",
    downloadingBytes: "Downloading {{bytes}}",
    downloading: "Downloading…",
    onDisk: "On disk",
    notDownloaded: "Not downloaded",
    remove: "Remove",
    removing: "Removing…",
    download: "Download",
    modelsFootnote:
      "Files come from Hugging Face (ggerganov/whisper.cpp). Handy models such as Nemotron, Parakeet, Voxtral, Qwen3-ASR, Fun-ASR, and Cohere Transcribe need transcribe.cpp, which this app does not run.",
    credentials: "Credentials",
    apiKey: "AssemblyAI API key",
    apiKeyLabel: "API key",
    configured: "Configured",
    notSet: "Not set",
    keepKey: "•••••••• (leave blank to keep)",
    enterKey: "Enter API key",
    save: "Save settings",
    saving: "Saving…",
    saved: "Settings saved.",
    downloaded: "Downloaded {{name}} ({{size}}). Select it and save to use it on Whisper jobs.",
    removed: "Removed {{id}} from this machine.",
    workspaceRequired: "Your workspace",
  },
  status: {
    queued: "Queued",
    processing: "Processing",
    completed: "Completed",
    failed: "Failed",
    cancelled: "Cancelled",
    unknown: "Unknown",
  },
};

const ptBR: typeof en = {
  sidebar: {
    ariaLabel: "Barra lateral do workspace",
    uploads: "Uploads",
    jobs: "Jobs",
    settings: "Configurações",
    language: "Idioma",
    onThisMachine: "Neste computador · sem conta",
  },
  language: {
    en: "English",
    "pt-BR": "Português (Brasil)",
  },
  theme: {
    light: "Claro",
    dark: "Escuro",
    switchToLight: "Mudar para modo claro",
    switchToDark: "Mudar para modo escuro",
  },
  upload: {
    eyebrow: "Palco",
    title: "Traga a fita",
    subtitle:
      "Solte áudio ou vídeo. Cada arquivo vira um job com o motor padrão das Configurações.",
    viewJobs: "Ver jobs",
    queuedOne: "1 job na fila de transcrição.",
    queuedMany: "{{count}} jobs na fila de transcrição.",
    openGroup: "Abrir grupo de upload",
    jobLabel: "Job #{{id}}",
    dropActive: "Solte para enfileirar",
    dropIdle: "Solte os arquivos neste deck",
    formats: "MP3, WAV, OGG, M4A, FLAC ou MP4, MOV, WebM, MKV · até 20 arquivos",
    browse: "Escolher arquivos",
    creating: "Criando jobs…",
    cannotUpload: "Não dá para enviar este arquivo",
    unsupportedType:
      "Este tipo de arquivo não é suportado. Use áudios comuns ou vídeo MP4, MOV, WebM e MKV.",
    dropFailed:
      "O drop falhou. Use Escolher arquivos, ou solte os arquivos com a janela do app em foco.",
  },
  jobs: {
    eyebrow: "Fila",
    title: "Jobs",
    subtitle: "Acompanhe as corridas, abra transcrições prontas, tente de novo as que falharam.",
    newUpload: "Novo upload",
    total: "Total",
    queued: "Na fila",
    processing: "Processando",
    completed: "Concluídos",
    failed: "Falhou",
    emptyTitle: "A bancada está vazia",
    emptyText: "Solte um arquivo no Palco para cortar a primeira transcrição.",
    audios: "{{count}} áudios",
  },
  job: {
    loading: "Carregando…",
    back: "Voltar para jobs",
    notFound: "Job não encontrado",
    batch: "Lote #{{id}}",
    provider: "Provider",
    status: "Status",
    output: "Saída",
    file: "Arquivo",
    retry: "Tentar de novo",
    retrying: "Tentando…",
    cancel: "Cancelar",
    cancelling: "Cancelando…",
    downloadMd: "Baixar markdown",
    empty: "Ainda sem transcrição.",
    manuscript: "Manuscrito",
    copy: "Copiar texto",
    copied: "Copiado",
    copyFailed: "Não foi possível copiar o texto.",
  },
  batch: {
    title: "Rolo · {{count}} arquivos",
    subtitle: "Troque as takes abaixo. Cada arquivo é um corte.",
    downloadAll: "Baixar todos",
    preparing: "Preparando…",
    tabs: "Arquivos do lote",
    openFull: "Abrir página completa",
    notFound: "Lote não encontrado. Abra de novo em Jobs.",
    viewJobs: "Ver jobs",
  },
  settings: {
    eyebrow: "Cabine",
    title: "Configurações",
    lede: "Escolha o motor, deixe as chaves neste computador e baixe pesos do Whisper só quando pedir.",
    workspace: "Workspace",
    workspaceHint: "App local · sem login multi-usuário",
    assemblyNote: "O AssemblyAI usa a chave de API guardada no SQLite local.",
    whisperNote:
      "O Whisper é local e não usa essa chave. Os modelos ficam no disco até você apagar.",
    provider: "Provider",
    defaultProvider: "Provider padrão",
    transcriptionLanguage: "Idioma da transcrição",
    langAuto: "Detectar automaticamente",
    langPt: "Português",
    langEn: "Inglês",
    langEs: "Espanhol",
    models: "Modelos Whisper locais",
    modelsLede:
      "Baixe só os tamanhos que quiser. Nada é baixado até você clicar em Download. Só dá para selecionar e salvar um modelo depois que ele estiver no disco.",
    modelMissing: "O modelo `{{id}}` ainda não foi baixado.",
    cannotSelect: "Baixe este modelo antes de selecioná-lo.",
    cannotSave:
      "Baixe um modelo Whisper e selecione-o antes de salvar o Whisper como provider padrão.",
    downloadingPct: "Baixando {{pct}}%",
    downloadingBytes: "Baixando {{bytes}}",
    downloading: "Baixando…",
    onDisk: "No disco",
    notDownloaded: "Não baixado",
    remove: "Remover",
    removing: "Removendo…",
    download: "Download",
    modelsFootnote:
      "Arquivos vêm do Hugging Face (ggerganov/whisper.cpp). Modelos do Handy como Nemotron, Parakeet, Voxtral, Qwen3-ASR, Fun-ASR e Cohere Transcribe precisam de transcribe.cpp, que este app não roda.",
    credentials: "Credenciais",
    apiKey: "Chave de API AssemblyAI",
    apiKeyLabel: "Chave de API",
    configured: "Configurada",
    notSet: "Não definida",
    keepKey: "•••••••• (deixe em branco para manter)",
    enterKey: "Cole a chave de API",
    save: "Salvar configurações",
    saving: "Salvando…",
    saved: "Configurações salvas.",
    downloaded:
      "Baixado {{name}} ({{size}}). Selecione e salve para usar nos jobs Whisper.",
    removed: "Removido {{id}} deste computador.",
    workspaceRequired: "Seu workspace",
  },
  status: {
    queued: "Na fila",
    processing: "Processando",
    completed: "Concluído",
    failed: "Falhou",
    cancelled: "Cancelado",
    unknown: "Desconhecido",
  },
};

const catalogs: Record<Locale, typeof en> = {
  en,
  "pt-BR": ptBR,
};

export function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "en" || stored === "pt-BR") return stored;
  const nav = navigator.language.toLowerCase();
  return nav.startsWith("pt") ? "pt-BR" : "en";
}

export function persistLocale(locale: Locale) {
  localStorage.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale === "pt-BR" ? "pt-BR" : "en";
}

type Vars = Record<string, string | number>;

export function translate(locale: Locale, key: string, vars?: Vars): string {
  const parts = key.split(".");
  let cur: unknown = catalogs[locale];
  for (const part of parts) {
    if (cur && typeof cur === "object" && part in cur) {
      cur = (cur as Record<string, unknown>)[part];
    } else {
      cur = undefined;
      break;
    }
  }
  let text = typeof cur === "string" ? cur : key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.replace(new RegExp(`\\{\\{${name}\\}\\}`, "g"), String(value));
    }
  }
  return text;
}
