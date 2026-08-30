import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import {
  detectLocale,
  persistLocale,
  translate,
  type Locale,
} from "./i18n";

type LocaleContextValue = {
  locale: Locale;
  setLocale: (next: Locale) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

const LocaleContext = createContext<LocaleContextValue | null>(null);

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => {
    const initial = detectLocale();
    persistLocale(initial);
    return initial;
  });

  const value = useMemo<LocaleContextValue>(
    () => ({
      locale,
      setLocale: (next) => {
        persistLocale(next);
        setLocaleState(next);
      },
      t: (key, vars) => translate(locale, key, vars),
    }),
    [locale],
  );

  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>;
}

export function useLocale() {
  const ctx = useContext(LocaleContext);
  if (!ctx) {
    throw new Error("useLocale must be used inside LocaleProvider");
  }
  return ctx;
}
