# Consola

Un escritorio para Windows que se maneja entero con mando, como una PS5 o el modo Big Picture de Steam. Sustituye al escritorio de Windows al arrancar.

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

## Volver al escritorio normal

Desde la consola: **Escritorio → Volver al modo normal de Windows**. Si algo falla, ejecuta `consola.exe --restaurar-escritorio`.
