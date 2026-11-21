//! Idiomas de la consola (lado Rust): textos que llegan a la pantalla, al móvil o a Windows.
//! El idioma lo elige la interfaz con `set_locale` al arrancar; hasta entonces se usa el de Windows.
//! Orden de las columnas: es, en, pt-BR, fr, de, it.
//! `tr("clave")` devuelve el texto; `trf("clave", &[("err", "...")])` cambia los {nombre}.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Es = 0,
    En = 1,
    PtBr = 2,
    Fr = 3,
    De = 4,
    It = 5,
}

const UNSET: u8 = u8::MAX;
static LANG: AtomicU8 = AtomicU8::new(UNSET);

const LANGS: [Lang; 6] = [Lang::Es, Lang::En, Lang::PtBr, Lang::Fr, Lang::De, Lang::It];
const CODES: [&str; 6] = ["es", "en", "pt-BR", "fr", "de", "it"];

/// "es-MX", "pt_BR", "EN"… → idioma conocido
fn from_code(code: &str) -> Option<Lang> {
    let c = code.trim().to_ascii_lowercase();
    let prefix = c.split(['-', '_', '.']).next().unwrap_or("");
    Some(match prefix {
        "es" => Lang::Es,
        "en" => Lang::En,
        "pt" => Lang::PtBr,
        "fr" => Lang::Fr,
        "de" => Lang::De,
        "it" => Lang::It,
        _ => return None,
    })
}

/// Idioma de Windows (o de LANG). Uno que no tenemos → inglés. En las pruebas, siempre español.
fn detect() -> Lang {
    if cfg!(test) {
        return Lang::Es;
    }
    let tag = windows::Globalization::ApplicationLanguages::Languages()
        .ok()
        .and_then(|l| l.GetAt(0).ok())
        .map(|h| h.to_string())
        .filter(|t| !t.is_empty())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();
    if tag.is_empty() { Lang::Es } else { from_code(&tag).unwrap_or(Lang::En) }
}

/// Cambia el idioma ("es", "en", "pt-BR", "fr", "de", "it"). Uno desconocido no cambia nada.
pub fn set(code: &str) {
    if let Some(l) = from_code(code) {
        LANG.store(l as u8, Ordering::Relaxed);
    }
}

pub fn current() -> Lang {
    let v = LANG.load(Ordering::Relaxed);
    if v == UNSET {
        let l = detect();
        LANG.store(l as u8, Ordering::Relaxed);
        return l;
    }
    LANGS.get(v as usize).copied().unwrap_or(Lang::Es)
}

/// Código del idioma actual: "es", "en", "pt-BR", "fr", "de", "it"
pub fn code() -> &'static str {
    CODES[current() as usize]
}

/// Nombre del idioma para las instrucciones de Claude ("Responde siempre en …")
pub fn language_name() -> &'static str {
    match current() {
        Lang::Es => "español",
        Lang::En => "inglés (English)",
        Lang::PtBr => "portugués de Brasil (português do Brasil)",
        Lang::Fr => "francés (français)",
        Lang::De => "alemán (Deutsch)",
        Lang::It => "italiano (italiano)",
    }
}

/// Texto de una clave en el idioma actual, si existe
pub fn lookup(key: &str) -> Option<&'static str> {
    T.iter().find(|(k, _)| *k == key).map(|(_, v)| v[current() as usize])
}

/// Texto de una clave en el idioma actual. Clave desconocida → la propia clave.
pub fn tr(key: &'static str) -> &'static str {
    lookup(key).unwrap_or(key)
}

/// Igual que `tr`, cambiando cada {nombre} por su valor
pub fn trf(key: &'static str, args: &[(&str, &str)]) -> String {
    let mut s = tr(key).to_string();
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

/// La interfaz avisa del idioma elegido (al arrancar y al cambiarlo en Ajustes)
#[tauri::command]
pub fn set_locale(code: String) {
    set(&code);
}

/// Catálogo: clave → [es, en, pt-BR, fr, de, it]
const T: &[(&str, [&str; 6])] = &[
    // ---------------- Comunes ----------------
    ("common.bad_action", ["Acción no válida", "Invalid action", "Ação inválida", "Action non valide", "Ungültige Aktion", "Azione non valida"]),
    ("common.no_internet", ["No hay conexión a internet.", "No internet connection.", "Sem conexão com a internet.", "Pas de connexion internet.", "Keine Internetverbindung.", "Nessuna connessione a internet."]),

    // ---------------- Archivos ----------------
    ("files.disk", ["Disco {letter}:", "Drive {letter}:", "Disco {letter}:", "Disque {letter}:", "Laufwerk {letter}:", "Disco {letter}:"]),
    ("files.received", ["Recibidos", "Received", "Recebidos", "Reçus", "Empfangen", "Ricevuti"]),
    ("files.downloads", ["Descargas", "Downloads", "Downloads", "Téléchargements", "Downloads", "Download"]),
    ("files.games", ["Juegos", "Games", "Jogos", "Jeux", "Spiele", "Giochi"]),
    ("files.desktop", ["Escritorio", "Desktop", "Área de trabalho", "Bureau", "Desktop", "Desktop"]),
    ("files.documents", ["Documentos", "Documents", "Documentos", "Documents", "Dokumente", "Documenti"]),
    ("files.pictures", ["Imágenes", "Pictures", "Imagens", "Images", "Bilder", "Immagini"]),
    ("files.videos", ["Vídeos", "Videos", "Vídeos", "Vidéos", "Videos", "Video"]),
    ("files.music", ["Música", "Music", "Músicas", "Musique", "Musik", "Musica"]),
    ("files.cant_open", ["No se puede abrir la carpeta: {err}", "Can't open the folder: {err}", "Não foi possível abrir a pasta: {err}", "Impossible d'ouvrir le dossier : {err}", "Der Ordner lässt sich nicht öffnen: {err}", "Impossibile aprire la cartella: {err}"]),
    ("files.not_found", ["Ya no existe", "It no longer exists", "Não existe mais", "Ça n'existe plus", "Existiert nicht mehr", "Non esiste più"]),
    ("files.dest_missing", ["La carpeta de destino no existe", "The destination folder doesn't exist", "A pasta de destino não existe", "Le dossier de destination n'existe pas", "Der Zielordner existiert nicht", "La cartella di destinazione non esiste"]),
    ("files.bad_path", ["Ruta no válida", "Invalid path", "Caminho inválido", "Chemin non valide", "Ungültiger Pfad", "Percorso non valido"]),
    ("files.into_itself", ["No se puede copiar una carpeta dentro de sí misma", "A folder can't be copied into itself", "Não dá para copiar uma pasta para dentro dela mesma", "Impossible de copier un dossier dans lui-même", "Ein Ordner kann nicht in sich selbst kopiert werden", "Non puoi copiare una cartella dentro sé stessa"]),
    ("files.new_folder", ["Nueva carpeta", "New folder", "Nova pasta", "Nouveau dossier", "Neuer Ordner", "Nuova cartella"]),

    // ---------------- Ajustes ----------------
    ("settings.plan_not_found", ["Plan de energía no encontrado", "Power plan not found", "Plano de energia não encontrado", "Mode d'alimentation introuvable", "Energiesparplan nicht gefunden", "Combinazione per il risparmio di energia non trovata"]),
    ("settings.wifi_not_saved", ["Esa red no está guardada en este PC", "That network isn't saved on this PC", "Essa rede não está salva neste PC", "Ce réseau n'est pas enregistré sur ce PC", "Dieses Netzwerk ist auf diesem PC nicht gespeichert", "Questa rete non è salvata su questo PC"]),

    // ---------------- Fondos ----------------
    ("backdrop.unknown_kind", ["Tipo de fondo desconocido: {kind}", "Unknown background type: {kind}", "Tipo de fundo desconhecido: {kind}", "Type de fond inconnu : {kind}", "Unbekannte Hintergrundart: {kind}", "Tipo di sfondo sconosciuto: {kind}"]),
    ("backdrop.saved", ["Fondos guardados", "Saved backgrounds", "Fundos salvos", "Fonds enregistrés", "Gespeicherte Hintergründe", "Sfondi salvati"]),
    ("backdrop.missing", ["No se encuentra el archivo. Puede que se haya movido o borrado.", "Can't find the file. It may have been moved or deleted.", "Arquivo não encontrado. Talvez tenha sido movido ou apagado.", "Fichier introuvable. Il a peut-être été déplacé ou supprimé.", "Datei nicht gefunden. Vielleicht wurde sie verschoben oder gelöscht.", "File non trovato. Forse è stato spostato o eliminato."]),
    ("backdrop.bad_type", ["Este tipo de archivo no sirve de fondo. Usa una imagen, un vídeo MP4 o WebM, o un modelo GLB.", "This file type can't be a background. Use an image, an MP4 or WebM video, or a GLB model.", "Esse tipo de arquivo não serve de fundo. Use uma imagem, um vídeo MP4 ou WebM, ou um modelo GLB.", "Ce type de fichier ne peut pas servir de fond. Utilise une image, une vidéo MP4 ou WebM, ou un modèle GLB.", "Dieser Dateityp geht nicht als Hintergrund. Nimm ein Bild, ein MP4- oder WebM-Video oder ein GLB-Modell.", "Questo tipo di file non va bene come sfondo. Usa un'immagine, un video MP4 o WebM, o un modello GLB."]),
    ("backdrop.copy_failed", ["No se pudo copiar el archivo: {err}. Comprueba que hay espacio libre en el disco.", "Couldn't copy the file: {err}. Check there's free space on the disk.", "Não foi possível copiar o arquivo: {err}. Verifique se há espaço livre no disco.", "Impossible de copier le fichier : {err}. Vérifie qu'il reste de la place sur le disque.", "Die Datei ließ sich nicht kopieren: {err}. Prüf, ob auf dem Laufwerk Platz frei ist.", "Impossibile copiare il file: {err}. Controlla che ci sia spazio libero sul disco."]),
    ("backdrop.model_unreadable", ["No se pudo leer el modelo: {err}", "Couldn't read the model: {err}", "Não foi possível ler o modelo: {err}", "Impossible de lire le modèle : {err}", "Das Modell ließ sich nicht lesen: {err}", "Impossibile leggere il modello: {err}"]),
    ("backdrop.model_part_missing", ["Falta el archivo «{file}» junto al modelo. Prueba con la versión .glb, que va en un solo archivo.", "The file “{file}” is missing next to the model. Try the .glb version, which comes in a single file.", "Falta o arquivo “{file}” junto ao modelo. Tente a versão .glb, que vem num arquivo só.", "Il manque le fichier « {file} » à côté du modèle. Essaie la version .glb, en un seul fichier.", "Die Datei „{file}“ fehlt neben dem Modell. Probier die .glb-Version, die aus einer einzigen Datei besteht.", "Manca il file «{file}» accanto al modello. Prova la versione .glb, che è in un unico file."]),

    // ---------------- Música ----------------
    ("media.player", ["Reproductor multimedia", "Media Player", "Media Player", "Lecteur multimédia", "Mediaplayer", "Lettore multimediale"]),
    ("media.player_short", ["Reproductor", "Player", "Player", "Lecteur", "Player", "Lettore"]),
    ("media.nothing_playing", ["No suena nada ahora mismo.", "Nothing is playing right now.", "Nada está tocando agora.", "Rien ne joue en ce moment.", "Gerade läuft nichts.", "Non sta suonando niente."]),
    ("media.bad_action", ["Acción no válida. Usa play, pause, toggle, next o previous.", "Invalid action. Use play, pause, toggle, next or previous.", "Ação inválida. Use play, pause, toggle, next ou previous.", "Action non valide. Utilise play, pause, toggle, next ou previous.", "Ungültige Aktion. Nutze play, pause, toggle, next oder previous.", "Azione non valida. Usa play, pause, toggle, next o previous."]),
    ("media.no_app", ["No hay ninguna app de música abierta.", "No music app is open.", "Nenhum app de música está aberto.", "Aucune appli de musique n'est ouverte.", "Keine Musik-App ist geöffnet.", "Nessuna app di musica è aperta."]),
    ("media.refused", ["La app de música no ha aceptado la orden.", "The music app didn't accept the command.", "O app de música não aceitou o comando.", "L'appli de musique n'a pas accepté la commande.", "Die Musik-App hat den Befehl nicht angenommen.", "L'app di musica non ha accettato il comando."]),
    ("media.no_spotify", ["Spotify no está instalado en este PC.", "Spotify isn't installed on this PC.", "O Spotify não está instalado neste PC.", "Spotify n'est pas installé sur ce PC.", "Spotify ist auf diesem PC nicht installiert.", "Spotify non è installato su questo PC."]),

    // ---------------- El tiempo ----------------
    ("weather.clear", ["Despejado", "Clear", "Céu limpo", "Dégagé", "Klar", "Sereno"]),
    ("weather.mostly_clear", ["Casi despejado", "Mostly clear", "Quase limpo", "Plutôt dégagé", "Überwiegend klar", "Quasi sereno"]),
    ("weather.partly_cloudy", ["Algo nuboso", "Partly cloudy", "Parcialmente nublado", "Partiellement nuageux", "Teilweise bewölkt", "Parzialmente nuvoloso"]),
    ("weather.cloudy", ["Nublado", "Cloudy", "Nublado", "Nuageux", "Bewölkt", "Nuvoloso"]),
    ("weather.fog", ["Niebla", "Fog", "Neblina", "Brouillard", "Nebel", "Nebbia"]),
    ("weather.drizzle", ["Llovizna", "Drizzle", "Garoa", "Bruine", "Nieselregen", "Pioviggine"]),
    ("weather.freezing_drizzle", ["Llovizna helada", "Freezing drizzle", "Garoa congelante", "Bruine verglaçante", "Gefrierender Nieselregen", "Pioviggine gelata"]),
    ("weather.light_rain", ["Lluvia débil", "Light rain", "Chuva fraca", "Pluie faible", "Leichter Regen", "Pioggia debole"]),
    ("weather.rain", ["Lluvia", "Rain", "Chuva", "Pluie", "Regen", "Pioggia"]),
    ("weather.heavy_rain", ["Lluvia fuerte", "Heavy rain", "Chuva forte", "Forte pluie", "Starker Regen", "Pioggia forte"]),
    ("weather.freezing_rain", ["Lluvia helada", "Freezing rain", "Chuva congelante", "Pluie verglaçante", "Gefrierender Regen", "Pioggia gelata"]),
    ("weather.light_snow", ["Nieve débil", "Light snow", "Neve fraca", "Neige faible", "Leichter Schneefall", "Neve debole"]),
    ("weather.snow", ["Nieve", "Snow", "Neve", "Neige", "Schnee", "Neve"]),
    ("weather.heavy_snow", ["Nieve fuerte", "Heavy snow", "Neve forte", "Forte neige", "Starker Schneefall", "Neve forte"]),
    ("weather.snow_grains", ["Granizo fino", "Snow grains", "Grãos de neve", "Neige en grains", "Griesel", "Neve granulosa"]),
    ("weather.showers", ["Chubascos", "Showers", "Pancadas de chuva", "Averses", "Schauer", "Rovesci"]),
    ("weather.heavy_showers", ["Chubascos fuertes", "Heavy showers", "Pancadas fortes", "Fortes averses", "Starke Schauer", "Rovesci forti"]),
    ("weather.snow_showers", ["Chubascos de nieve", "Snow showers", "Pancadas de neve", "Averses de neige", "Schneeschauer", "Rovesci di neve"]),
    ("weather.storm", ["Tormenta", "Thunderstorm", "Tempestade", "Orage", "Gewitter", "Temporale"]),
    ("weather.storm_hail", ["Tormenta con granizo", "Thunderstorm with hail", "Tempestade com granizo", "Orage avec grêle", "Gewitter mit Hagel", "Temporale con grandine"]),
    ("weather.variable", ["Tiempo variable", "Changeable weather", "Tempo variável", "Temps variable", "Wechselhaftes Wetter", "Tempo variabile"]),
    ("weather.bad_reply", ["El servicio del tiempo respondió algo raro.", "The weather service sent back something odd.", "O serviço do tempo respondeu algo estranho.", "Le service météo a renvoyé une réponse bizarre.", "Der Wetterdienst hat etwas Seltsames geantwortet.", "Il servizio meteo ha risposto qualcosa di strano."]),
    ("weather.down", ["El servicio del tiempo no responde ahora. Prueba más tarde.", "The weather service isn't responding right now. Try later.", "O serviço do tempo não está respondendo agora. Tente mais tarde.", "Le service météo ne répond pas pour l'instant. Réessaie plus tard.", "Der Wetterdienst antwortet gerade nicht. Versuch es später.", "Il servizio meteo ora non risponde. Riprova più tardi."]),
    ("weather.no_data", ["El servicio del tiempo no dio datos.", "The weather service gave no data.", "O serviço do tempo não deu dados.", "Le service météo n'a donné aucune donnée.", "Der Wetterdienst hat keine Daten geliefert.", "Il servizio meteo non ha dato dati."]),
    ("weather.no_temp", ["Falta la temperatura.", "The temperature is missing.", "Falta a temperatura.", "Il manque la température.", "Die Temperatur fehlt.", "Manca la temperatura."]),
    ("weather.no_location", ["No se pudo calcular tu ubicación. Elige una ciudad.", "Couldn't work out your location. Pick a city.", "Não foi possível descobrir sua localização. Escolha uma cidade.", "Impossible de trouver ta position. Choisis une ville.", "Dein Standort ließ sich nicht ermitteln. Wähl eine Stadt.", "Impossibile calcolare la tua posizione. Scegli una città."]),
    ("weather.pick_city", ["Elige tu ciudad en Ajustes → Salvapantallas.", "Pick your city in Settings → Screensaver.", "Escolha sua cidade em Configurações → Proteção de tela.", "Choisis ta ville dans Réglages → Économiseur d'écran.", "Wähl deine Stadt unter Einstellungen → Bildschirmschoner.", "Scegli la tua città in Impostazioni → Salvaschermo."]),

    // ---------------- Voz ----------------
    ("voice.privacy", ["Windows no deja reconocer la voz. Activa «Reconocimiento de voz en línea» en Configuración de Windows › Privacidad › Voz.", "Windows won't allow speech recognition. Turn on “Online speech recognition” in Windows Settings › Privacy & security › Speech.", "O Windows não permite reconhecer a voz. Ative “Reconhecimento de fala online” em Configurações do Windows › Privacidade e segurança › Fala.", "Windows bloque la reconnaissance vocale. Active « Reconnaissance vocale en ligne » dans Paramètres Windows › Confidentialité et sécurité › Voix.", "Windows blockiert die Spracherkennung. Aktiviere „Onlinespracherkennung“ unter Windows-Einstellungen › Datenschutz und Sicherheit › Spracherkennung.", "Windows non permette il riconoscimento vocale. Attiva «Riconoscimento vocale online» in Impostazioni di Windows › Privacy e sicurezza › Voce."]),
    ("voice.no_language", ["Falta el paquete de voz en español. Instálalo en Configuración de Windows › Hora e idioma › Voz.", "The English speech pack is missing. Install it in Windows Settings › Time & language › Speech.", "Falta o pacote de fala em português. Instale em Configurações do Windows › Hora e idioma › Fala.", "Le pack vocal français est manquant. Installe-le dans Paramètres Windows › Heure et langue › Voix.", "Das deutsche Sprachpaket fehlt. Installiere es unter Windows-Einstellungen › Zeit und Sprache › Spracherkennung.", "Manca il pacchetto vocale italiano. Installalo in Impostazioni di Windows › Data/ora e lingua › Voce."]),
    ("voice.mic_blocked", ["Windows no deja usar el micrófono. Actívalo en Configuración de Windows › Privacidad › Micrófono.", "Windows won't allow the microphone. Turn it on in Windows Settings › Privacy & security › Microphone.", "O Windows não permite usar o microfone. Ative em Configurações do Windows › Privacidade e segurança › Microfone.", "Windows bloque le micro. Active-le dans Paramètres Windows › Confidentialité et sécurité › Microphone.", "Windows blockiert das Mikrofon. Aktiviere es unter Windows-Einstellungen › Datenschutz und Sicherheit › Mikrofon.", "Windows non permette di usare il microfono. Attivalo in Impostazioni di Windows › Privacy e sicurezza › Microfono."]),
    ("voice.no_mic", ["No encuentro ningún micrófono. Conecta uno y vuelve a probar.", "I can't find a microphone. Plug one in and try again.", "Não encontrei nenhum microfone. Conecte um e tente de novo.", "Je ne trouve aucun micro. Branches-en un et réessaie.", "Ich finde kein Mikrofon. Schließ eins an und versuch es noch mal.", "Non trovo nessun microfono. Collegane uno e riprova."]),
    ("voice.no_network", ["No hay conexión a internet para reconocer la voz. Revisa el Wi-Fi.", "No internet connection for speech recognition. Check your Wi-Fi.", "Sem conexão com a internet para reconhecer a voz. Verifique o Wi-Fi.", "Pas de connexion internet pour reconnaître la voix. Vérifie le Wi-Fi.", "Keine Internetverbindung für die Spracherkennung. Prüf das WLAN.", "Nessuna connessione a internet per riconoscere la voce. Controlla il Wi-Fi."]),
    ("voice.failed", ["No pude escuchar (código {code}). Prueba otra vez.", "I couldn't listen (code {code}). Try again.", "Não consegui ouvir (código {code}). Tente de novo.", "Je n'ai pas pu écouter (code {code}). Réessaie.", "Zuhören hat nicht geklappt (Code {code}). Versuch es noch mal.", "Non sono riuscito ad ascoltare (codice {code}). Riprova."]),
    ("voice.timeout", ["El reconocimiento de voz no responde. Prueba otra vez.", "Speech recognition isn't responding. Try again.", "O reconhecimento de voz não responde. Tente de novo.", "La reconnaissance vocale ne répond pas. Réessaie.", "Die Spracherkennung reagiert nicht. Versuch es noch mal.", "Il riconoscimento vocale non risponde. Riprova."]),

    // ---------------- Salvapantallas, juegos, mandos, equipo ----------------
    ("idle.title", ["Salvapantallas", "Screensaver", "Proteção de tela", "Économiseur d'écran", "Bildschirmschoner", "Salvaschermo"]),
    ("idle.save_failed", ["No se pudieron guardar los ajustes.", "Couldn't save the settings.", "Não foi possível salvar as configurações.", "Impossible d'enregistrer les réglages.", "Die Einstellungen ließen sich nicht speichern.", "Impossibile salvare le impostazioni."]),
    ("games.not_found", ["No encontrado", "Not found", "Não encontrado", "Introuvable", "Nicht gefunden", "Non trovato"]),
    ("pads.xbox", ["Mando de Xbox", "Xbox controller", "Controle do Xbox", "Manette Xbox", "Xbox-Controller", "Controller Xbox"]),
    ("pads.ds4_adapter", ["DualShock 4 (adaptador)", "DualShock 4 (adapter)", "DualShock 4 (adaptador)", "DualShock 4 (adaptateur)", "DualShock 4 (Adapter)", "DualShock 4 (adattatore)"]),
    ("hw.cpu", ["Procesador", "Processor", "Processador", "Processeur", "Prozessor", "Processore"]),
    ("hw.gpu_unknown", ["Desconocida", "Unknown", "Desconhecida", "Inconnue", "Unbekannt", "Sconosciuta"]),

    // ---------------- Navegador (barra que se pone encima de las webs) ----------------
    ("browser.back", ["Atrás", "Back", "Voltar", "Retour", "Zurück", "Indietro"]),
    ("browser.open_windows", ["Abrir en Windows", "Open in Windows", "Abrir no Windows", "Ouvrir dans Windows", "In Windows öffnen", "Apri in Windows"]),
    ("browser.home", ["Volver a la consola", "Back to CouchDesk", "Voltar ao CouchDesk", "Retour à CouchDesk", "Zurück zu CouchDesk", "Torna a CouchDesk"]),
    ("browser.hold_esc", ["o mantén Esc", "or hold Esc", "ou segure Esc", "ou maintiens Échap", "oder Esc halten", "o tieni premuto Esc"]),
    ("browser.bad_url", ["Dirección no válida", "Invalid address", "Endereço inválido", "Adresse non valide", "Ungültige Adresse", "Indirizzo non valido"]),

    // ---------------- Móvil, envío de archivos, FTP, LocalSend ----------------
    ("remote.scan_qr", ["Escanea el QR de la consola", "Scan the QR code on CouchDesk", "Escaneie o QR do CouchDesk", "Scanne le QR code de CouchDesk", "Scanne den QR-Code von CouchDesk", "Scansiona il QR di CouchDesk"]),
    ("remote.app_name", ["Mando de CouchDesk", "CouchDesk Controller", "Controle do CouchDesk", "Manette CouchDesk", "CouchDesk-Controller", "Controller CouchDesk"]),
    ("remote.app_short", ["Mando", "Controller", "Controle", "Manette", "Controller", "Controller"]),
    ("remote.phone", ["Móvil {n}", "Phone {n}", "Celular {n}", "Téléphone {n}", "Handy {n}", "Telefono {n}"]),
    ("transfer.saves", ["Partidas", "Saves", "Saves", "Sauvegardes", "Spielstände", "Salvataggi"]),
    ("transfer.unknown_folder", ["Carpeta desconocida", "Unknown folder", "Pasta desconhecida", "Dossier inconnu", "Unbekannter Ordner", "Cartella sconosciuta"]),
    ("transfer.bad_name", ["Nombre de archivo no válido", "Invalid file name", "Nome de arquivo inválido", "Nom de fichier non valide", "Ungültiger Dateiname", "Nome del file non valido"]),
    ("transfer.cant_create", ["No se puede crear la carpeta: {err}", "Can't create the folder: {err}", "Não foi possível criar a pasta: {err}", "Impossible de créer le dossier : {err}", "Der Ordner lässt sich nicht erstellen: {err}", "Impossibile creare la cartella: {err}"]),
    ("transfer.cut", ["Se cortó la conexión", "The connection dropped", "A conexão caiu", "La connexion a été coupée", "Die Verbindung wurde unterbrochen", "La connessione si è interrotta"]),
    ("receive.greeting", ["CouchDesk - Recibir archivos", "CouchDesk - Receive files", "CouchDesk - Receber arquivos", "CouchDesk - Recevoir des fichiers", "CouchDesk - Dateien empfangen", "CouchDesk - Ricevi file"]),
    ("receive.port_busy", ["El puerto {port} está ocupado", "Port {port} is in use", "A porta {port} está ocupada", "Le port {port} est occupé", "Port {port} ist belegt", "La porta {port} è occupata"]),
    ("localsend.port_busy", ["El puerto {port} está ocupado (¿LocalSend abierto en este PC?): {err}", "Port {port} is in use (is LocalSend open on this PC?): {err}", "A porta {port} está ocupada (o LocalSend está aberto neste PC?): {err}", "Le port {port} est occupé (LocalSend est ouvert sur ce PC ?) : {err}", "Port {port} ist belegt (ist LocalSend auf diesem PC offen?): {err}", "La porta {port} è occupata (LocalSend è aperto su questo PC?): {err}"]),
    ("localsend.bad_names", ["Nombres de archivo no válidos", "Invalid file names", "Nomes de arquivo inválidos", "Noms de fichiers non valides", "Ungültige Dateinamen", "Nomi dei file non validi"]),

    // ---------------- Emuladores ----------------
    ("emu.no_connection", ["Sin conexión con {url}: {err}", "Can't reach {url}: {err}", "Sem conexão com {url}: {err}", "Impossible de joindre {url} : {err}", "Keine Verbindung zu {url}: {err}", "Nessuna connessione con {url}: {err}"]),
    ("emu.http_status", ["{url} respondió {status}", "{url} replied {status}", "{url} respondeu {status}", "{url} a répondu {status}", "{url} antwortete {status}", "{url} ha risposto {status}"]),
    ("emu.no_releases", ["Este proyecto no tiene versiones publicadas", "This project has no published versions", "Este projeto não tem versões publicadas", "Ce projet n'a aucune version publiée", "Dieses Projekt hat keine veröffentlichten Versionen", "Questo progetto non ha versioni pubblicate"]),
    ("emu.no_windows", ["No hay versión para Windows", "There's no Windows version", "Não há versão para Windows", "Pas de version pour Windows", "Keine Version für Windows", "Non c'è una versione per Windows"]),
    ("emu.no_dolphin", ["No se encontró la versión de Dolphin", "Couldn't find the Dolphin version", "Não foi possível encontrar a versão do Dolphin", "Version de Dolphin introuvable", "Dolphin-Version nicht gefunden", "Versione di Dolphin non trovata"]),
    ("emu.download_failed", ["La descarga falló ({status})", "The download failed ({status})", "O download falhou ({status})", "Le téléchargement a échoué ({status})", "Der Download ist fehlgeschlagen ({status})", "Il download non è riuscito ({status})"]),
    ("emu.no_exe", ["No se encontró el programa dentro de la descarga", "Couldn't find the program inside the download", "Não foi possível encontrar o programa no download", "Programme introuvable dans le téléchargement", "Das Programm wurde im Download nicht gefunden", "Programma non trovato nel download"]),
    ("emu.unknown", ["Emulador desconocido", "Unknown emulator", "Emulador desconhecido", "Émulateur inconnu", "Unbekannter Emulator", "Emulatore sconosciuto"]),
    ("emu.not_installed", ["No está instalado", "It's not installed", "Não está instalado", "Il n'est pas installé", "Nicht installiert", "Non è installato"]),
    ("emu.no_previous", ["No hay versión anterior", "There's no previous version", "Não há versão anterior", "Pas de version précédente", "Keine vorherige Version", "Non c'è una versione precedente"]),
    ("emu.folder.games", ["Tus juegos", "Your games", "Seus jogos", "Tes jeux", "Deine Spiele", "I tuoi giochi"]),
    ("emu.folder.games_hint", ["Copia aquí tus juegos. La primera vez que abras el emulador, elige esta carpeta como carpeta de juegos.", "Copy your games here. The first time you open the emulator, pick this folder as your games folder.", "Copie seus jogos aqui. Na primeira vez que abrir o emulador, escolha esta pasta como pasta de jogos.", "Copie tes jeux ici. La première fois que tu ouvres l'émulateur, choisis ce dossier comme dossier de jeux.", "Kopier deine Spiele hierher. Wähl beim ersten Start des Emulators diesen Ordner als Spieleordner.", "Copia qui i tuoi giochi. La prima volta che apri l'emulatore, scegli questa cartella come cartella dei giochi."]),
    ("emu.folder.bios", ["BIOS, firmware y claves", "BIOS, firmware and keys", "BIOS, firmware e chaves", "BIOS, firmware et clés", "BIOS, Firmware und Schlüssel", "BIOS, firmware e chiavi"]),
    ("emu.folder.bios_duckstation", ["BIOS de tu PS1 (por ejemplo scph5501.bin). DuckStation la encuentra sola.", "The BIOS from your PS1 (for example scph5501.bin). DuckStation finds it on its own.", "BIOS do seu PS1 (por exemplo scph5501.bin). O DuckStation encontra sozinho.", "Le BIOS de ta PS1 (par exemple scph5501.bin). DuckStation le trouve tout seul.", "Das BIOS deiner PS1 (zum Beispiel scph5501.bin). DuckStation findet es von selbst.", "Il BIOS della tua PS1 (per esempio scph5501.bin). DuckStation lo trova da solo."]),
    ("emu.folder.bios_pcsx2", ["BIOS de tu PS2 (archivo .bin). PCSX2 la encuentra sola.", "The BIOS from your PS2 (.bin file). PCSX2 finds it on its own.", "BIOS do seu PS2 (arquivo .bin). O PCSX2 encontra sozinho.", "Le BIOS de ta PS2 (fichier .bin). PCSX2 le trouve tout seul.", "Das BIOS deiner PS2 (.bin-Datei). PCSX2 findet es von selbst.", "Il BIOS della tua PS2 (file .bin). PCSX2 lo trova da solo."]),
    ("emu.folder.bios_melonds", ["bios7.bin, bios9.bin y firmware.bin. Después elígelos en melonDS: Ajustes → Emulación.", "bios7.bin, bios9.bin and firmware.bin. Then pick them in melonDS: Config → Emu settings.", "bios7.bin, bios9.bin e firmware.bin. Depois escolha no melonDS: Config → Emu settings.", "bios7.bin, bios9.bin et firmware.bin. Ensuite, choisis-les dans melonDS : Config → Emu settings.", "bios7.bin, bios9.bin und firmware.bin. Danach in melonDS auswählen: Config → Emu settings.", "bios7.bin, bios9.bin e firmware.bin. Poi sceglili in melonDS: Config → Emu settings."]),
    ("emu.folder.bios_rpcs3", ["Deja aquí PS3UPDAT.PUP y, en RPCS3, Archivo → Instalar firmware.", "Put PS3UPDAT.PUP here, then in RPCS3 use File → Install Firmware.", "Deixe o PS3UPDAT.PUP aqui e, no RPCS3, use File → Install Firmware.", "Mets PS3UPDAT.PUP ici puis, dans RPCS3, File → Install Firmware.", "Leg PS3UPDAT.PUP hier ab und wähl in RPCS3 File → Install Firmware.", "Metti qui PS3UPDAT.PUP e in RPCS3 usa File → Install Firmware."]),
    ("emu.folder.bios_shadps4", ["Opcional: módulos de sistema de tu PS4.", "Optional: system modules from your PS4.", "Opcional: módulos de sistema do seu PS4.", "En option : les modules système de ta PS4.", "Optional: Systemmodule deiner PS4.", "Facoltativo: i moduli di sistema della tua PS4."]),
    ("emu.folder.bios_eden", ["prod.keys y title.keys de tu consola. El firmware se instala desde el propio emulador.", "prod.keys and title.keys from your console. The firmware is installed from the emulator itself.", "prod.keys e title.keys do seu console. O firmware é instalado pelo próprio emulador.", "prod.keys et title.keys de ta console. Le firmware s'installe depuis l'émulateur.", "prod.keys und title.keys deiner Konsole. Die Firmware installierst du im Emulator selbst.", "prod.keys e title.keys della tua console. Il firmware si installa dall'emulatore stesso."]),
    ("emu.folder.data", ["Partidas y ajustes", "Saves and settings", "Saves e configurações", "Sauvegardes et réglages", "Spielstände und Einstellungen", "Salvataggi e impostazioni"]),
    ("emu.folder.data_hint", ["Tus partidas guardadas y la configuración del emulador.", "Your saved games and the emulator settings.", "Seus jogos salvos e as configurações do emulador.", "Tes sauvegardes et les réglages de l'émulateur.", "Deine Spielstände und die Einstellungen des Emulators.", "I tuoi salvataggi e le impostazioni dell'emulatore."]),
    // Catálogo (emu.<id>.bios / note / console)
    ("emu.duckstation.bios", ["Necesita la BIOS de tu PS1 (por ejemplo scph5501.bin). Cópiala en la carpeta «bios» del emulador; el asistente del primer arranque la busca allí.", "Needs the BIOS from your PS1 (for example scph5501.bin). Copy it to the emulator's “bios” folder; the first-run wizard looks for it there.", "Precisa da BIOS do seu PS1 (por exemplo scph5501.bin). Copie para a pasta “bios” do emulador; o assistente da primeira execução procura lá.", "Il faut le BIOS de ta PS1 (par exemple scph5501.bin). Copie-le dans le dossier « bios » de l'émulateur ; l'assistant du premier lancement le cherche là.", "Braucht das BIOS deiner PS1 (zum Beispiel scph5501.bin). Kopier es in den Ordner „bios“ des Emulators; der Einrichtungsassistent sucht dort danach.", "Serve il BIOS della tua PS1 (per esempio scph5501.bin). Copialo nella cartella «bios» dell'emulatore; la procedura del primo avvio lo cerca lì."]),
    ("emu.pcsx2.bios", ["Necesita la BIOS de tu PS2 (archivo .bin sacado de tu consola). Ponla en la carpeta «bios»; PCSX2 la detecta en el asistente inicial.", "Needs the BIOS from your PS2 (a .bin file dumped from your console). Put it in the “bios” folder; PCSX2 finds it in the setup wizard.", "Precisa da BIOS do seu PS2 (arquivo .bin extraído do seu console). Coloque na pasta “bios”; o PCSX2 detecta no assistente inicial.", "Il faut le BIOS de ta PS2 (fichier .bin extrait de ta console). Mets-le dans le dossier « bios » ; PCSX2 le détecte dans l'assistant de départ.", "Braucht das BIOS deiner PS2 (.bin-Datei aus deiner Konsole). Leg es in den Ordner „bios“; PCSX2 erkennt es im Einrichtungsassistenten.", "Serve il BIOS della tua PS2 (file .bin estratto dalla tua console). Mettilo nella cartella «bios»; PCSX2 lo rileva nella procedura iniziale."]),
    ("emu.rpcs3.bios", ["Necesita el firmware oficial PS3UPDAT.PUP, gratis en la web de PlayStation (actualización del sistema de PS3). En RPCS3: Archivo → Instalar firmware.", "Needs the official PS3UPDAT.PUP firmware, free on the PlayStation website (PS3 system update). In RPCS3: File → Install Firmware.", "Precisa do firmware oficial PS3UPDAT.PUP, grátis no site da PlayStation (atualização do sistema do PS3). No RPCS3: File → Install Firmware.", "Il faut le firmware officiel PS3UPDAT.PUP, gratuit sur le site PlayStation (mise à jour système PS3). Dans RPCS3 : File → Install Firmware.", "Braucht die offizielle Firmware PS3UPDAT.PUP, kostenlos auf der PlayStation-Website (PS3-Systemupdate). In RPCS3: File → Install Firmware.", "Serve il firmware ufficiale PS3UPDAT.PUP, gratis sul sito PlayStation (aggiornamento di sistema PS3). In RPCS3: File → Install Firmware."]),
    ("emu.rpcs3.note", ["Tira mucho del procesador: los juegos ligeros van, los pesados no.", "Very CPU-heavy: light games run, heavy ones don't.", "Exige muito do processador: jogos leves rodam, os pesados não.", "Très gourmand en processeur : les jeux légers tournent, les lourds non.", "Braucht viel CPU-Leistung: Leichte Spiele laufen, schwere nicht.", "Pesa molto sul processore: i giochi leggeri vanno, quelli pesanti no."]),
    ("emu.shadps4.bios", ["No necesita BIOS. Opcional: módulos de sistema de tu PS4 en la carpeta «sys_modules».", "No BIOS needed. Optional: system modules from your PS4 in the “sys_modules” folder.", "Não precisa de BIOS. Opcional: módulos de sistema do seu PS4 na pasta “sys_modules”.", "Pas besoin de BIOS. En option : les modules système de ta PS4 dans le dossier « sys_modules ».", "Kein BIOS nötig. Optional: Systemmodule deiner PS4 im Ordner „sys_modules“.", "Non serve il BIOS. Facoltativo: i moduli di sistema della tua PS4 nella cartella «sys_modules»."]),
    ("emu.shadps4.note", ["Pocos juegos jugables todavía.", "Few playable games so far.", "Poucos jogos jogáveis por enquanto.", "Peu de jeux jouables pour l'instant.", "Bisher nur wenige spielbare Spiele.", "Ancora pochi giochi giocabili."]),
    ("emu.kytyps5.bios", ["No necesita BIOS ni firmware.", "No BIOS or firmware needed.", "Não precisa de BIOS nem firmware.", "Pas besoin de BIOS ni de firmware.", "Kein BIOS und keine Firmware nötig.", "Non servono BIOS né firmware."]),
    ("emu.sharpemu.bios", ["No necesita BIOS. También abre algunos juegos de PS4.", "No BIOS needed. It also runs some PS4 games.", "Não precisa de BIOS. Também abre alguns jogos de PS4.", "Pas besoin de BIOS. Il lance aussi certains jeux PS4.", "Kein BIOS nötig. Startet auch einige PS4-Spiele.", "Non serve il BIOS. Apre anche alcuni giochi PS4."]),
    ("emu.anyps5.bios", ["No necesita BIOS.", "No BIOS needed.", "Não precisa de BIOS.", "Pas besoin de BIOS.", "Kein BIOS nötig.", "Non serve il BIOS."]),
    ("emu.anyps5.note", ["No es un emulador: convierte el juego en un programa de Windows.", "Not an emulator: it turns the game into a Windows program.", "Não é um emulador: transforma o jogo em um programa do Windows.", "Ce n'est pas un émulateur : il transforme le jeu en programme Windows.", "Kein Emulator: Er macht aus dem Spiel ein Windows-Programm.", "Non è un emulatore: trasforma il gioco in un programma di Windows."]),
    ("emu.ares.bios", ["No necesita BIOS para Nintendo 64.", "No BIOS needed for Nintendo 64.", "Não precisa de BIOS para Nintendo 64.", "Pas besoin de BIOS pour la Nintendo 64.", "Für Nintendo 64 ist kein BIOS nötig.", "Non serve il BIOS per Nintendo 64."]),
    ("emu.melonds.bios", ["Opcional: bios7.bin, bios9.bin y firmware.bin de tu DS (Ajustes → Emulación). Sin ellos usa una alternativa libre.", "Optional: bios7.bin, bios9.bin and firmware.bin from your DS (Config → Emu settings). Without them it uses a free alternative.", "Opcional: bios7.bin, bios9.bin e firmware.bin do seu DS (Config → Emu settings). Sem eles, usa uma alternativa livre.", "En option : bios7.bin, bios9.bin et firmware.bin de ta DS (Config → Emu settings). Sans eux, il utilise une alternative libre.", "Optional: bios7.bin, bios9.bin und firmware.bin deines DS (Config → Emu settings). Ohne sie nutzt er eine freie Alternative.", "Facoltativo: bios7.bin, bios9.bin e firmware.bin del tuo DS (Config → Emu settings). Senza, usa un'alternativa libera."]),
    ("emu.dolphin.bios", ["No necesita BIOS.", "No BIOS needed.", "Não precisa de BIOS.", "Pas besoin de BIOS.", "Kein BIOS nötig.", "Non serve il BIOS."]),
    ("emu.dolphin.console", ["Wii y GameCube", "Wii & GameCube", "Wii e GameCube", "Wii et GameCube", "Wii und GameCube", "Wii e GameCube"]),
    ("emu.eden.bios", ["Necesita prod.keys y el firmware de TU Switch. En Eden: Herramientas → Instalar claves / Instalar firmware.", "Needs prod.keys and the firmware from YOUR Switch. In Eden: Tools → Install Keys / Install Firmware.", "Precisa do prod.keys e do firmware do SEU Switch. No Eden: Tools → Install Keys / Install Firmware.", "Il faut prod.keys et le firmware de TA Switch. Dans Eden : Tools → Install Keys / Install Firmware.", "Braucht prod.keys und die Firmware DEINER Switch. In Eden: Tools → Install Keys / Install Firmware.", "Servono prod.keys e il firmware della TUA Switch. In Eden: Tools → Install Keys / Install Firmware."]),
    ("emu.novas.bios", ["Necesita prod.keys (y title.keys) y el firmware de TU Switch 2: botones «Claves» y «Firmware» del propio NovaS.", "Needs prod.keys (and title.keys) and the firmware from YOUR Switch 2: use the “Keys” and “Firmware” buttons in NovaS.", "Precisa do prod.keys (e title.keys) e do firmware do SEU Switch 2: botões “Keys” e “Firmware” do próprio NovaS.", "Il faut prod.keys (et title.keys) et le firmware de TA Switch 2 : boutons « Keys » et « Firmware » de NovaS.", "Braucht prod.keys (und title.keys) und die Firmware DEINER Switch 2: Buttons „Keys“ und „Firmware“ in NovaS.", "Servono prod.keys (e title.keys) e il firmware della TUA Switch 2: pulsanti «Keys» e «Firmware» di NovaS."]),
    ("emu.novas.note", ["Proyecto pequeño y nuevo: úsalo con cuidado.", "Small, new project: use it with care.", "Projeto pequeno e novo: use com cuidado.", "Petit projet tout neuf : utilise-le avec prudence.", "Kleines, neues Projekt: Benutz es mit Vorsicht.", "Progetto piccolo e nuovo: usalo con cautela."]),
    ("emu.eden-nx2.bios", ["Necesita prod.keys y el firmware de TU Switch 2 (Herramientas → Instalar claves / firmware).", "Needs prod.keys and the firmware from YOUR Switch 2 (Tools → Install Keys / Firmware).", "Precisa do prod.keys e do firmware do SEU Switch 2 (Tools → Install Keys / Firmware).", "Il faut prod.keys et le firmware de TA Switch 2 (Tools → Install Keys / Firmware).", "Braucht prod.keys und die Firmware DEINER Switch 2 (Tools → Install Keys / Firmware).", "Servono prod.keys e il firmware della TUA Switch 2 (Tools → Install Keys / Firmware)."]),
    ("emu.eden-nx2.note", ["Rama experimental de Eden.", "Experimental branch of Eden.", "Ramo experimental do Eden.", "Branche expérimentale d'Eden.", "Experimenteller Zweig von Eden.", "Ramo sperimentale di Eden."]),

    // ---------------- Claude ----------------
    ("claude.not_logged_in", ["Claude Code no tiene la sesión iniciada. En el escritorio de Windows abre «Terminal», escribe claude y entra con tu cuenta.", "Claude Code isn't signed in. On the Windows desktop, open “Terminal”, type claude and sign in with your account.", "O Claude Code não está conectado. Na área de trabalho do Windows, abra o “Terminal”, digite claude e entre com sua conta.", "Claude Code n'est pas connecté. Sur le bureau Windows, ouvre « Terminal », tape claude et connecte-toi avec ton compte.", "Claude Code ist nicht angemeldet. Öffne auf dem Windows-Desktop „Terminal“, tippe claude ein und melde dich mit deinem Konto an.", "Claude Code non ha effettuato l'accesso. Sul desktop di Windows apri «Terminale», scrivi claude ed entra con il tuo account."]),
    ("claude.usage_limit", ["Has llegado al límite de uso de tu cuenta de Claude. Prueba más tarde.", "You've hit the usage limit on your Claude account. Try again later.", "Você atingiu o limite de uso da sua conta do Claude. Tente mais tarde.", "Tu as atteint la limite d'utilisation de ton compte Claude. Réessaie plus tard.", "Du hast das Nutzungslimit deines Claude-Kontos erreicht. Versuch es später noch mal.", "Hai raggiunto il limite di utilizzo del tuo account Claude. Riprova più tardi."]),
    ("claude.no_network", ["No hay conexión con Claude. Revisa el Wi-Fi y vuelve a probar.", "Can't reach Claude. Check your Wi-Fi and try again.", "Sem conexão com o Claude. Verifique o Wi-Fi e tente de novo.", "Impossible de joindre Claude. Vérifie le Wi-Fi et réessaie.", "Keine Verbindung zu Claude. Prüf das WLAN und versuch es noch mal.", "Nessuna connessione con Claude. Controlla il Wi-Fi e riprova."]),
    ("claude.conv_gone", ["No encuentro esa conversación. Empieza una nueva.", "I can't find that conversation. Start a new one.", "Não encontrei essa conversa. Comece uma nova.", "Je ne trouve pas cette conversation. Commence-en une nouvelle.", "Ich finde dieses Gespräch nicht. Starte ein neues.", "Non trovo quella conversazione. Iniziane una nuova."]),
    ("claude.problem", ["Claude tuvo un problema: {err}", "Claude ran into a problem: {err}", "O Claude teve um problema: {err}", "Claude a eu un problème : {err}", "Claude hatte ein Problem: {err}", "Claude ha avuto un problema: {err}"]),
    ("claude.conversation", ["Conversación", "Conversation", "Conversa", "Conversation", "Gespräch", "Conversazione"]),
    ("claude.conv_not_found", ["No encuentro esa conversación", "I can't find that conversation", "Não encontrei essa conversa", "Je ne trouve pas cette conversation", "Ich finde dieses Gespräch nicht", "Non trovo quella conversazione"]),
    ("claude.bad_url", ["Esa dirección no es válida. Debe empezar por https://", "That address isn't valid. It must start with https://", "Esse endereço não é válido. Ele deve começar com https://", "Cette adresse n'est pas valide. Elle doit commencer par https://", "Diese Adresse ist ungültig. Sie muss mit https:// beginnen", "Questo indirizzo non è valido. Deve iniziare con https://"]),
    ("claude.connector_exists", ["Ese conector ya está añadido", "That connector is already added", "Esse conector já foi adicionado", "Ce connecteur est déjà ajouté", "Dieser Connector ist schon hinzugefügt", "Questo connettore è già aggiunto"]),
    ("claude.connector_not_found", ["No encuentro ese conector", "I can't find that connector", "Não encontrei esse conector", "Je ne trouve pas ce connecteur", "Ich finde diesen Connector nicht", "Non trovo quel connettore"]),
    ("claude.not_installed", ["Claude Code no está instalado", "Claude Code isn't installed", "O Claude Code não está instalado", "Claude Code n'est pas installé", "Claude Code ist nicht installiert", "Claude Code non è installato"]),
    ("claude.cant_start", ["No se pudo abrir Claude: {err}", "Couldn't open Claude: {err}", "Não foi possível abrir o Claude: {err}", "Impossible d'ouvrir Claude : {err}", "Claude ließ sich nicht öffnen: {err}", "Impossibile aprire Claude: {err}"]),
    ("claude.no_reply", ["Claude no respondió. Vuelve a probar.", "Claude didn't answer. Try again.", "O Claude não respondeu. Tente de novo.", "Claude n'a pas répondu. Réessaie.", "Claude hat nicht geantwortet. Versuch es noch mal.", "Claude non ha risposto. Riprova."]),
    ("claude.timeout", ["Claude Code tardó demasiado. Vuelve a probar.", "Claude Code took too long. Try again.", "O Claude Code demorou demais. Tente de novo.", "Claude Code a mis trop de temps. Réessaie.", "Claude Code hat zu lange gebraucht. Versuch es noch mal.", "Claude Code ci ha messo troppo. Riprova."]),
    ("claude.plugins_failed", ["No pude ver las extensiones. {err}", "I couldn't load the extensions. {err}", "Não consegui ver as extensões. {err}", "Je n'ai pas pu voir les extensions. {err}", "Ich konnte die Erweiterungen nicht laden. {err}", "Non sono riuscito a vedere le estensioni. {err}"]),
    ("claude.failed", ["No se pudo hacer.", "That didn't work.", "Não foi possível fazer isso.", "Ça n'a pas marché.", "Das hat nicht geklappt.", "Non è stato possibile farlo."]),
    ("claude.bad_plugin", ["Nombre de extensión no válido", "Invalid extension name", "Nome de extensão inválido", "Nom d'extension non valide", "Ungültiger Erweiterungsname", "Nome dell'estensione non valido"]),
    ("claude.plugin_installed", ["Extensión instalada", "Extension installed", "Extensão instalada", "Extension installée", "Erweiterung installiert", "Estensione installata"]),
    ("claude.plugin_removed", ["Extensión quitada", "Extension removed", "Extensão removida", "Extension retirée", "Erweiterung entfernt", "Estensione rimossa"]),
    ("claude.assistant_title", ["Asistente", "Assistant", "Assistente", "Assistant", "Assistent", "Assistente"]),
    // Tienda de conectores (claude.cat.<nombre>)
    ("claude.cat.deepwiki", ["Explica proyectos de GitHub y responde dudas sobre ellos.", "Explains GitHub projects and answers questions about them.", "Explica projetos do GitHub e tira dúvidas sobre eles.", "Explique des projets GitHub et répond à tes questions.", "Erklärt GitHub-Projekte und beantwortet Fragen dazu.", "Spiega progetti GitHub e risponde ai dubbi su di essi."]),
    ("claude.cat.context7", ["Documentación al día de miles de programas y librerías.", "Up-to-date docs for thousands of programs and libraries.", "Documentação atualizada de milhares de programas e bibliotecas.", "Documentation à jour de milliers de programmes et bibliothèques.", "Aktuelle Doku zu Tausenden Programmen und Bibliotheken.", "Documentazione aggiornata di migliaia di programmi e librerie."]),
    ("claude.cat.microsoft-learn", ["Ayuda oficial de Windows, Xbox, Office y más.", "Official help for Windows, Xbox, Office and more.", "Ajuda oficial do Windows, Xbox, Office e mais.", "Aide officielle pour Windows, Xbox, Office et plus.", "Offizielle Hilfe zu Windows, Xbox, Office und mehr.", "Guida ufficiale di Windows, Xbox, Office e altro."]),
    ("claude.cat.huggingface", ["Busca modelos y demos de inteligencia artificial.", "Find AI models and demos.", "Busca modelos e demos de inteligência artificial.", "Cherche des modèles et démos d'intelligence artificielle.", "Sucht KI-Modelle und Demos.", "Cerca modelli e demo di intelligenza artificiale."]),
    ("claude.cat.cloudflare-docs", ["Ayuda de Cloudflare: WARP, DNS y webs.", "Cloudflare help: WARP, DNS and websites.", "Ajuda da Cloudflare: WARP, DNS e sites.", "Aide Cloudflare : WARP, DNS et sites web.", "Cloudflare-Hilfe: WARP, DNS und Websites.", "Guida di Cloudflare: WARP, DNS e siti web."]),
    ("claude.cat.gitmcp", ["Lee la documentación de cualquier proyecto de GitHub.", "Reads the docs of any GitHub project.", "Lê a documentação de qualquer projeto do GitHub.", "Lit la documentation de n'importe quel projet GitHub.", "Liest die Doku jedes GitHub-Projekts.", "Legge la documentazione di qualsiasi progetto GitHub."]),
    // Herramientas de la consola que usa Claude ("Claude está usando …"; claves tool.<nombre>)
    ("tool.ver_volumen", ["ver volumen", "check volume", "ver volume", "voir le volume", "Lautstärke ansehen", "vedere il volume"]),
    ("tool.poner_volumen", ["poner volumen", "set volume", "ajustar volume", "régler le volume", "Lautstärke einstellen", "impostare il volume"]),
    ("tool.silenciar", ["silenciar", "mute", "silenciar", "couper le son", "stummschalten", "silenziare"]),
    ("tool.ver_brillo", ["ver brillo", "check brightness", "ver brilho", "voir la luminosité", "Helligkeit ansehen", "vedere la luminosità"]),
    ("tool.poner_brillo", ["poner brillo", "set brightness", "ajustar brilho", "régler la luminosité", "Helligkeit einstellen", "impostare la luminosità"]),
    ("tool.ver_bateria", ["ver batería", "check battery", "ver bateria", "voir la batterie", "Akku ansehen", "vedere la batteria"]),
    ("tool.ver_modos_energia", ["ver modos de energía", "check power modes", "ver modos de energia", "voir les modes d'alimentation", "Energiemodi ansehen", "vedere le modalità di energia"]),
    ("tool.poner_modo_energia", ["poner modo de energía", "set power mode", "mudar modo de energia", "changer le mode d'alimentation", "Energiemodus einstellen", "cambiare modalità di energia"]),
    ("tool.ver_wifi", ["ver Wi-Fi", "check Wi-Fi", "ver Wi-Fi", "voir le Wi-Fi", "WLAN ansehen", "vedere il Wi-Fi"]),
    ("tool.ver_hardware", ["ver hardware", "check hardware", "ver hardware", "voir le matériel", "Hardware ansehen", "vedere l'hardware"]),
    ("tool.listar_juegos", ["listar juegos", "list games", "listar jogos", "lister les jeux", "Spiele auflisten", "elencare i giochi"]),
    ("tool.abrir_juego", ["abrir juego", "open game", "abrir jogo", "ouvrir un jeu", "Spiel öffnen", "aprire un gioco"]),
    ("tool.listar_emuladores", ["listar emuladores", "list emulators", "listar emuladores", "lister les émulateurs", "Emulatoren auflisten", "elencare gli emulatori"]),
    ("tool.instalar_emulador", ["instalar emulador", "install emulator", "instalar emulador", "installer un émulateur", "Emulator installieren", "installare un emulatore"]),
    ("tool.abrir_emulador", ["abrir emulador", "open emulator", "abrir emulador", "ouvrir un émulateur", "Emulator öffnen", "aprire un emulatore"]),
    ("tool.listar_carpeta", ["listar carpeta", "list folder", "listar pasta", "lister un dossier", "Ordner auflisten", "elencare una cartella"]),
    ("tool.lugares", ["lugares", "places", "locais", "emplacements", "Orte", "posizioni"]),
    ("tool.abrir_archivo", ["abrir archivo", "open file", "abrir arquivo", "ouvrir un fichier", "Datei öffnen", "aprire un file"]),
    ("tool.abrir_pantalla", ["abrir pantalla", "open screen", "abrir tela", "ouvrir un écran", "Bildschirm öffnen", "aprire una schermata"]),
    ("tool.abrir_web", ["abrir web", "open website", "abrir site", "ouvrir un site", "Website öffnen", "aprire un sito"]),
    ("tool.buscar_en_youtube", ["buscar en YouTube", "search YouTube", "buscar no YouTube", "chercher sur YouTube", "auf YouTube suchen", "cercare su YouTube"]),
    ("tool.listar_aplicaciones", ["listar aplicaciones", "list apps", "listar aplicativos", "lister les applis", "Apps auflisten", "elencare le app"]),
    ("tool.abrir_aplicacion", ["abrir aplicación", "open app", "abrir aplicativo", "ouvrir une appli", "App öffnen", "aprire un'app"]),
    ("tool.volver_al_inicio", ["volver al inicio", "go home", "voltar ao início", "revenir à l'accueil", "zum Start zurück", "tornare alla home"]),
    ("tool.musica_que_suena", ["música que suena", "now playing", "música tocando", "musique en cours", "läuft gerade", "musica in riproduzione"]),
    ("tool.musica_control", ["control de música", "music control", "controle de música", "contrôle de la musique", "Musiksteuerung", "controllo della musica"]),
    ("tool.musica_ver_volumen", ["ver volumen de la música", "check music volume", "ver volume da música", "voir le volume de la musique", "Musiklautstärke ansehen", "vedere il volume della musica"]),
    ("tool.musica_poner_volumen", ["poner volumen de la música", "set music volume", "ajustar volume da música", "régler le volume de la musique", "Musiklautstärke einstellen", "impostare il volume della musica"]),
    ("tool.abrir_spotify", ["abrir Spotify", "open Spotify", "abrir Spotify", "ouvrir Spotify", "Spotify öffnen", "aprire Spotify"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Nombres entre llaves de un texto: "Error {err} en {url}" → ["err", "url"]
    fn placeholders(s: &str) -> Vec<String> {
        let mut out: Vec<String> = s
            .split('{')
            .skip(1)
            .filter_map(|p| p.split_once('}').map(|(n, _)| n.to_string()))
            .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn todas_las_claves_tienen_seis_textos() {
        for (key, v) in T {
            for (i, text) in v.iter().enumerate() {
                assert!(!text.trim().is_empty(), "{key}: falta el texto en {}", CODES[i]);
            }
        }
    }

    #[test]
    fn mismas_llaves_en_todos_los_idiomas() {
        for (key, v) in T {
            let es = placeholders(v[0]);
            for (i, text) in v.iter().enumerate().skip(1) {
                assert_eq!(placeholders(text), es, "{key}: no coinciden los {{nombres}} en {}", CODES[i]);
            }
        }
    }

    #[test]
    fn sin_claves_repetidas() {
        let mut keys: Vec<&str> = T.iter().map(|(k, _)| *k).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n, "hay claves repetidas");
    }

    /// Toda clave escrita en el código ("files.not_found"…) existe en el catálogo
    #[test]
    fn claves_usadas_en_el_codigo_existen() {
        const PREFIXES: &[&str] = &[
            "common.", "files.", "settings.", "backdrop.", "media.", "weather.", "voice.", "idle.", "games.", "pads.", "hw.",
            "browser.", "remote.", "transfer.", "receive.", "localsend.", "emu.", "claude.", "tool.",
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") || path.file_name().is_some_and(|n| n == "i18n.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (i, _) in text.match_indices('"') {
                let rest = &text[i + 1..];
                if !PREFIXES.iter().any(|p| rest.starts_with(p)) {
                    continue;
                }
                let Some(end) = rest.find('"') else { continue };
                let key = &rest[..end];
                let is_key = key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c));
                // Nombres de archivo que empiezan igual que una clave
                const NOT_KEYS: &[&str] = &["claude.exe", "settings.ini"];
                if is_key && !key.ends_with('.') && !NOT_KEYS.contains(&key) && !T.iter().any(|(k, _)| *k == key) {
                    missing.push(format!("{}: {key}", path.file_name().unwrap().to_string_lossy()));
                }
            }
        }
        assert!(missing.is_empty(), "claves sin traducir: {missing:?}");
    }

    #[test]
    fn cambiar_idioma() {
        assert_eq!(from_code("pt-BR"), Some(Lang::PtBr));
        assert_eq!(from_code("pt_PT"), Some(Lang::PtBr));
        assert_eq!(from_code("EN-us"), Some(Lang::En));
        assert_eq!(from_code("ja-JP"), None);
        // Español por defecto en las pruebas
        assert_eq!(tr("files.new_folder"), "Nueva carpeta");
        assert_eq!(trf("files.cant_open", &[("err", "x")]), "No se puede abrir la carpeta: x");
        assert_eq!(tr("no.existe"), "no.existe");
        assert_eq!(lookup("no.existe"), None);
        assert!(language_name().starts_with("español"));
    }
}
