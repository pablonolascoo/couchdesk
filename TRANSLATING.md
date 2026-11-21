# Translating CouchDesk

CouchDesk speaks Spanish (the base), English, Portuguese (Brazil), French, German and Italian.
Adding a language takes no coding skills beyond editing text files. Thank you for helping!

## Where the text lives

| Part | Where | Format |
|---|---|---|
| App screens | `src/i18n/locales/<code>/*.json` | One JSON file per area (`home.json`, `settings.json`…). Flat keys, prefixed by the area: `"settings.title": "Settings"` |
| Messages from the PC side (errors, notices, Claude's language) | `src-tauri/src/i18n.rs` | A table: each key has one text per language, in a fixed order |
| Phone remote page | `src-tauri/remote/index.html`, `src-tauri/remote/subir.html` | A small dictionary inside the page (`DICT = { es: {…}, en: {…} }`) |

Spanish (`es`) is the reference. If a text is missing in your language, the app shows the Spanish one.

## Add a new language

1. Copy the folder `src/i18n/locales/es` to `src/i18n/locales/<code>` (for example `nl` or `pt-PT`).
2. Translate every value in every JSON file. Do not change the keys (the left side).
3. Add the language to `LANGUAGES` in `src/i18n/index.ts`, with its name in its own language:
   `{ code: 'nl', name: 'Nederlands', english: 'Dutch' }`.
4. PC-side messages: in `src-tauri/src/i18n.rs`, add the language to `Lang`, to `set()` and `language_name()`,
   and add one more text to every entry of the table. Run `cargo test --lib` in `src-tauri` to check it.
5. Phone page: add a block for your language in the dictionary of `src-tauri/remote/index.html` (and `subir.html`).
6. Optional: on-screen keyboard. Add a layout for your language in `LAYOUTS` (`src/components/Keyboard.tsx`)
   if it needs extra letters (like ñ, ß or ç).
7. Run `npm run i18n:check`. It lists missing keys, extra keys and placeholder mistakes. Fix them until it says
   "Todo en orden."
8. Open a pull request. Say which language you added and whether a native speaker reviewed it.

To fix or improve an existing language, just edit its files and open a pull request.

## Style rules

- **Short sentences.** One idea per sentence. The app is read from the sofa, on a TV.
- **No jargon.** Write for someone who has never configured a PC. "Couldn't open the file", not "I/O error".
- **Buttons start with a verb:** "Open", "Delete", "Try again".
- **Friendly and direct.** Talk to the person (informal "you" where your language allows it).
- **Keep placeholders as they are:** `{name}`, `{n}`, `{err}`… Move them anywhere in the sentence, but don't
  translate or remove them.
- **Plurals:** a text like `"{n} game|{n} games"` has two forms separated by `|`: singular first, then plural.
  Keep the `|`.
- **Brand names stay:** Steam, Xbox, YouTube, Claude, CouchDesk…
- **Watch the length.** Labels sit on buttons and narrow rows. If your translation is much longer than the
  Spanish one, look for a shorter word.
- **Dates, times and numbers** are formatted automatically for your language. Don't write them in the texts.

---

## Resumen en español

Para añadir un idioma:

1. Copia `src/i18n/locales/es` a `src/i18n/locales/<código>` y traduce los valores (no las claves).
2. Añade el idioma a `LANGUAGES` en `src/i18n/index.ts`.
3. Textos de la parte del PC: añade el idioma en `src-tauri/src/i18n.rs` (un texto más en cada entrada).
4. Página del móvil: añade tu idioma al diccionario de `src-tauri/remote/index.html` y `subir.html`.
5. Ejecuta `npm run i18n:check` hasta que diga «Todo en orden.».
6. Abre una pull request.

Reglas: frases cortas, sin jerga, botones que empiezan por un verbo, y deja las variables (`{nombre}`, `{n}`)
y la forma `uno|varios` tal cual.
