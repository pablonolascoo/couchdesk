# Registro de cambios

Todas las versiones de CouchDesk (antes «Consola»). Formato: [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) · versiones [SemVer](https://semver.org/lang/es/) con nombre en clave.

## [1.2.0] — Lino — 2026-11-21

Más rápida, con multitarea tipo PS5 y tu escritorio de Windows ordenado solo.

### Añadido
- **Multitarea:** pulsa el botón Xbox o PS (o Ctrl+Alt+Inicio) desde cualquier sitio y verás todo lo abierto. Cambia de app, ciérrala o libera memoria. Mantenlo pulsado para ir al Inicio.
- **Gestión de memoria como en iOS:** lo que queda detrás pasa a modo eficiencia y ocupa menos memoria. Lo que juegas recibe más prioridad. Todo se deshace al salir.
- **Escritorio organizado:** los iconos de tu escritorio de Windows se ordenan solos por zonas (Juegos, Aplicaciones, Documentos…), con iconos más grandes. Se puede volver al escritorio de antes con un botón.
- **Rendimiento:** modo juego, sin grabación de fondo de la Game Bar, máximo rendimiento al jugar y opciones para que Windows Update no moleste. Todo se puede deshacer.
- **LocalSend cifrado:** la consola recibe por HTTPS, como LocalSend envía por defecto. Usa un certificado propio que se crea una vez. Los envíos sin cifrar siguen funcionando en el mismo puerto.

### Mejorado
- **Mandos sin retardo:** CouchDesk lee los mandos cada 2 ms con su propio hilo (antes, hasta 33 ms).
- **Arranque más ligero:** la app carga la mitad de código al abrirse.
- **Animaciones siempre:** el fondo y las transiciones se mueven aunque Windows tenga las animaciones apagadas. Se pueden reducir en Ajustes → Rendimiento.
- **Vídeos a cámara lenta:** salvapantallas y modo ambiente con clips de 60 fotogramas a media velocidad, suaves y sin saltos.
- **Instalador CouchDesk:** con la marca, en 6 idiomas. Retira la antigua «Consola» y trae tus datos solo.
- **Envíos al doble de rápido o más:** la consola escribe a disco mientras recibe, sin esperas (de ~180 a ~480 MB/s en el propio PC). Los archivos grandes viajan en trozos por 6 conexiones a la vez, como un gestor de descargas, y los pequeños ya no esperan a que acabe el grande. LocalSend también se beneficia.

## [1.1.0] — Brasa — 2026-11-21

La consola se llama ahora **CouchDesk**, con marca propia, en 6 idiomas y con temas.

### Añadido
- **Marca CouchDesk:** logo «Standby» (una C con la luz de encendido), icono nuevo y arranque animado.
- **6 idiomas:** español, inglés, portugués (Brasil), francés, alemán e italiano. Usa el idioma del sistema y se cambia en Ajustes → Idioma. Cualquiera puede añadir el suyo (TRANSLATING.md).
- **Temas:** Salón, Noche, Cine, Bosque, Costa y Alto contraste. Puedes crear el tuyo y compartirlo con un código.
- **Música de menú:** suave e infinita, creada en el momento. Se calla sola al abrir un juego, una web, Spotify o un vídeo.
- **Fondo animado** tipo PS5 que apenas gasta, o tu propia imagen, vídeo, vídeo de YouTube o modelo 3D.
- **Salvapantallas** con paisajes en movimiento, hora, fecha y el tiempo de tu ciudad. Modo bajo consumo. Puede ser el salvapantallas de Windows.
- **Modo ambiente:** chimenea, lluvia, cascada, olas, acuario, auroras… a pantalla completa.
- **Música:** Spotify o cualquier reproductor. Qué suena, pausa, siguiente y volumen solo de la música.
- **Mandos:** todos los conectados a la vez con su batería, prueba de botones, perfiles (estándar, estilo Nintendo, zurdo o el tuyo) y cambio de botones.
- **Teclado en pantalla** para escribir con el mando, con las letras de cada idioma.
- **Claude como agente:** voz, conversaciones guardadas, conectores, extensiones y ventana flotante encima de todo (Ctrl+Alt+Espacio o Vista+Y). Abre pantallas, webs, apps y controla la música.
- **Más accesos en el navegador, agrupados por tipo:**
  - Cine y series: Disney+, HBO Max, Apple TV, Movistar Plus+, SkyShowtime, Filmin, RTVE Play, Atresplayer, Mitele y Crunchyroll.
  - Deporte, directos y música: DAZN, Kick, Spotify y Apple Music.
  - Juegos en la nube: Boosteroid.
- **Pasar archivos a los emuladores:**
  - Botón «Traer» en cada carpeta del emulador (juegos, BIOS, partidas). Eliges un USB o cualquier carpeta, marcas los archivos y los copias o mueves allí.
  - «Traer archivos aquí» en cualquier carpeta del explorador. Los USB salen primero en la lista.
  - Enviar desde el navegador del móvil u otro PC: escaneas un QR, eliges el emulador y la carpeta, y arrastras archivos o carpetas enteras. Envía varios a la vez, sin instalar nada.
  - El FTP crea una carpeta por emulador («PCSX2 - BIOS»…). Lo que dejas ahí se coloca solo en su sitio.
- **Recibir con LocalSend:** la consola aparece sola en la app gratuita LocalSend del móvil o de otro PC.
  - Con la pantalla «Recibir con LocalSend» abierta, los envíos se aceptan solos. Si no, la consola se abre ahí y pregunta «Aceptar» o «Rechazar».
  - Eliges dónde se guarda: Recibidos o los juegos, la BIOS o las partidas de cada emulador.
  - Solo red de casa. Nombres saneados y nada se sobrescribe.

### Cambiado
- «Recibir archivos» es ahora un menú con tres opciones: navegador, FTP y LocalSend.
- El proyecto se llama ahora **CouchDesk**: [github.com/pablonolascoo/couchdesk](https://github.com/pablonolascoo/couchdesk).
- Transiciones nuevas en toda la app: entrada del Inicio, cambio de pantalla y foco del mando.
- Si la app cambia de carpeta al actualizarse, el arranque en CouchDesk se repara solo.

### Arreglado
- Al abrir la app ya no parpadea una ventana de terminal.
- En el Inicio, los nombres de las secciones ya no se cortan.

## [1.0.0] — Salón — 2026-11-21

Primera versión pública.

### Juegos y aplicaciones
- **Biblioteca única:**
  - Reúne los juegos de Steam, Epic, GOG y Xbox / Game Pass.
  - También los de EA, Ubisoft, Riot, Rockstar, Battle.net, itch o Amazon que aparecen en el menú Inicio.
  - Si alguno falta, se marca como juego; si sobra, se quita con «No es un juego».
- **Aplicaciones:** todo lo instalado en el PC, con su icono real. Las herramientas de Windows van aparte.
- **En la nube:**
  - Xbox Cloud Gaming a pantalla completa.
  - PS5 Remote Play con [chiaki-ng](https://github.com/chiaki-ng/chiaki-ng).
- **Emuladores:** se instalan, se actualizan solos y se puede volver a la versión anterior.

### Mandos
- **Mandos físicos:**
  - Xbox, PS4 y PS5, por cable o Bluetooth, sin drivers.
  - El touchpad de PlayStation hace de ratón.
  - El mando sirve de ratón y teclado en todo Windows: START + BACK, u OPTIONS + SHARE en PlayStation.
- **Mando en el móvil:**
  - Se conecta escaneando un QR.
  - Estilo Xbox o PS5, con LB/RB arriba y LT/RT abajo.
  - Touchpad y teclado en la misma pantalla.
  - La consola cambia sola el modo del móvil y le pide el teclado cuando hay que escribir.
- **Teclado y ratón:** también funcionan; la leyenda de abajo cambia sola.

### Navegador
- **Pantalla completa y manejable con mando**, con accesos directos a YouTube, Netflix, Prime Video, Amazon, Twitch, Xbox Cloud, GeForce NOW, Google y Wikipedia.
- **Cada acceso** lleva su logo y la imagen de su web.
- **Salida siempre a mano:**
  - Barra arriba.
  - **Mantener Esc 1 s**, o pulsarlo 3 veces.
  - El botón Xbox del mando.
  - El botón Inicio del móvil.

### Sistema
- **Arranque:** puede arrancar en lugar del escritorio de Windows y volver al modo normal con un botón.
- **Una sola consola a la vez:** si se abre otra, se trae al frente la que ya está.
- **Siempre fluida:** ninguna tarea bloquea la pantalla. La versión final sale con la optimización máxima.
- **Más funciones:**
  - Recibir archivos por FTP en la red de casa.
  - Explorador de archivos.
  - Ajustes de sonido, brillo, energía y Wi-Fi.
  - Asistente Claude.
