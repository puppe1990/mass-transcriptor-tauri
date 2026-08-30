import { useEffect, useState } from "react";
import { useLocale } from "../lib/LocaleContext";
import { IconMoon, IconSun } from "./icons";

type Theme = "dark" | "light";

function readTheme(): Theme {
  const stored = localStorage.getItem("mt-theme");
  if (stored === "light" || stored === "dark") return stored;
  return "dark";
}

function applyTheme(theme: Theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("mt-theme", theme);
}

export function ThemeToggle() {
  const { t } = useLocale();
  const [theme, setTheme] = useState<Theme>(() => readTheme());

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  const next = theme === "dark" ? "light" : "dark";
  const label = theme === "dark" ? t("theme.light") : t("theme.dark");

  return (
    <button
      type="button"
      id="theme-toggle"
      className="theme-toggle btn--ghost"
      aria-label={theme === "dark" ? t("theme.switchToLight") : t("theme.switchToDark")}
      onClick={() => setTheme(next)}
    >
      <span aria-hidden="true">{theme === "dark" ? <IconMoon /> : <IconSun />}</span>
      <span data-theme-label>{label}</span>
    </button>
  );
}
