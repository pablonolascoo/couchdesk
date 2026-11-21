# Consola

[![Descargar para Windows](https://img.shields.io/github/v/release/pablonolascoo/consola?label=Descargar%20para%20Windows&style=for-the-badge)](https://github.com/pablonolascoo/consola/releases/latest)
[![Licencia MIT](https://img.shields.io/badge/licencia-MIT-2f3437?style=for-the-badge)](LICENSE)

Un escritorio para Windows que se maneja entero con mando, como una PS5 o el modo Big Picture de Steam. Puede arrancar en lugar del escritorio de Windows.

**Web:** [pablonolascoo.github.io/consola](https://pablonolascoo.github.io/consola/) · **Descarga:** [última versión](https://github.com/pablonolascoo/consola/releases/latest) · **Cambios:** [CHANGELOG](CHANGELOG.md)

## Instalar

1. Descarga `Consola_X.Y.Z_x64-setup.exe` de la [última versión](https://github.com/pablonolascoo/consola/releases/latest).
2. Ábrelo. Si Windows avisa de que no conoce el programa (aún no está firmado): **Más información → Ejecutar de todas formas**.
3. Para que arranque en lugar del escritorio: **Ajustes → Arranque**. Para volver: **Escritorio → Volver al modo normal de Windows**.

## Volver a la consola desde cualquier sitio

- Mantén **Esc** 1 segundo, o púlsalo 3 veces seguidas.
- Pulsa el botón **Xbox** del mando, el **PS** del de PlayStation o **Inicio** en el móvil.

## Qué hace

- **Biblioteca única:** reúne los juegos de Steam, Epic, GOG, Xbox y los de otras tiendas que encuentra en el menú Inicio. Si alguno falta, lo marcas como juego.
- **Aplicaciones:** lista todo lo instalado, con su icono real.
- **En la nube:** Xbox Cloud Gaming y PS5 Remote Play (con [chiaki-ng](https://github.com/chiaki-ng/chiaki-ng)).
- **Emuladores:** los instala, los actualiza y deja volver a la versión anterior.
- **Mandos:**
  - Xbox, PS4 y PS5, por cable o Bluetooth.
  - El touchpad de PlayStation hace de ratón.
  - El mando hace de ratón y teclado en todo Windows.
- **Mando en el móvil:** escaneas un QR y el móvil se convierte en un mando, con sticks, touchpad y teclado.
- **Teclado y ratón:** también funcionan.
- **Más:** navegador manejable con mando, explorador de archivos, recibir archivos por FTP, ajustes de sonido, brillo, energía y Wi-Fi, y asistente Claude.

## Tecnología

[Tauri 2](https://tauri.app) (Rust) + React 19 + Vite. Solo Windows 10/11.

## Desarrollo

```bash
npm install
npm run tauri dev      # desarrollo
npm run app:build      # instalador en src-tauri/target/release/bundle/nsis
cd src-tauri && cargo test --lib
```

## Versiones

Cada versión tiene número ([SemVer](https://semver.org/lang/es/)) y nombre en clave: la primera es **Salón v1.0.0**. Para publicar una (hace falta [Bun](https://bun.sh) y [GitHub CLI](https://cli.github.com)):

```bash
npm run release          # 1.0.0 → 1.0.1
npm run release minor    # 1.0.1 → 1.1.0
npm run release major    # 1.1.0 → 2.0.0
```

Sube la versión, compila el instalador, crea la etiqueta, publica la versión en GitHub con el instalador y avisa a los buscadores (IndexNow). Si la compilación falla, lo deja todo como estaba. Antes, apunta los cambios en [CHANGELOG.md](CHANGELOG.md).

## Volver al escritorio normal

Desde la consola: **Escritorio → Volver al modo normal de Windows**. Si algo falla, ejecuta `consola.exe --restaurar-escritorio`.
