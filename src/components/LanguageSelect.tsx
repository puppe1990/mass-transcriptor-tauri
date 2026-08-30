import type { Locale } from "../lib/i18n";
import { useLocale } from "../lib/LocaleContext";

export function LanguageSelect() {
  const { locale, setLocale, t } = useLocale();

  return (
    <label className="app-sidebar__field">
      <span>{t("sidebar.language")}</span>
      <select
        id="sidebar-language"
        className="app-control app-control--select"
        aria-label={t("sidebar.language")}
        value={locale}
        onChange={(e) => setLocale(e.target.value as Locale)}
      >
        <option value="en">{t("language.en")}</option>
        <option value="pt-BR">{t("language.pt-BR")}</option>
      </select>
    </label>
  );
}
