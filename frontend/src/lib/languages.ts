/** The languages the sandbox supports, with their metadata and starter code. */
import type { Language } from "./api";

export interface LanguageDescriptor {
  /** The value used by the backend. */
  id: Language;
  /** The name shown in the UI. */
  label: string;
  /** The default file name used by the save dialog. */
  fileName: string;
  /** The extension of the default file name. */
  extension: string;
  /** A short starter program. */
  starter: string;
}

/** All supported languages, in the order they are offered. */
export const LANGUAGES: LanguageDescriptor[] = [
  {
    id: "rust",
    label: "Rust",
    fileName: "main.rs",
    extension: "rs",
    starter: `fn main() {
    let names = vec!["world", "sandbox"];

    for name in names {
        println!("Hello, {name}!");
    }
}
`,
  },
  {
    id: "python",
    label: "Python",
    fileName: "main.py",
    extension: "py",
    starter: `names = ["world", "sandbox"]

for name in names:
    print(f"Hello, {name}!")
`,
  },
  {
    id: "javascript",
    label: "JavaScript",
    fileName: "main.js",
    extension: "js",
    starter: `const names = ["world", "sandbox"];

for (const name of names) {
    console.log(\`Hello, \${name}!\`);
}
`,
  },
];

/** Looks up the descriptor of a language. */
export function describeLanguage(language: Language): LanguageDescriptor {
  return (
    LANGUAGES.find((entry) => entry.id === language) ?? LANGUAGES[0]
  );
}
