//! Hablar con Claude: voz a texto con el reconocimiento de Windows (Windows.Media.SpeechRecognition).
//! Mantener pulsado = escuchar. Al soltar llega el texto final.
//! Eventos (con la ventana que lo pidió, para no mezclar la principal y la flotante):
//! · "voice-partial" { window, text }  lo que va entendiendo
//! · "voice-final"   { window, text }  el texto completo al soltar
//! · "voice-error"   { window, text }  un problema durante la escucha

use serde::Serialize;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use windows::Foundation::TypedEventHandler;
use windows::Globalization::Language;
use windows::Media::SpeechRecognition::*;
use windows::core::HSTRING;

static STOP: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);

#[derive(Serialize, Clone)]
struct VoiceEvent {
    window: String,
    text: String,
}

/// Elige idioma: el del sistema si es español; si no, es-ES; si no, otro español; si no, el del sistema
#[cfg_attr(not(test), allow(dead_code))]
pub fn choose_language(supported: &[String], system: &str) -> Option<String> {
    choose_language_for(supported, system, "es", "es-ES")
}

/// Igual, para el idioma de la consola (`want` = "en", preferido `main` = "en-US"…)
pub fn choose_language_for(supported: &[String], system: &str, want: &str, main: &str) -> Option<String> {
    let has = |t: &str| supported.iter().find(|s| s.eq_ignore_ascii_case(t)).cloned();
    if system.to_lowercase().starts_with(want) {
        if let Some(s) = has(system) {
            return Some(s);
        }
    }
    has(main)
        .or_else(|| supported.iter().find(|s| s.to_lowercase().starts_with(&format!("{want}-"))).cloned())
        .or_else(|| has(system))
        .or_else(|| supported.first().cloned())
}

/// Traduce los fallos de Windows a algo que se entienda y diga cómo arreglarlo
pub fn explain(code: i32, status: Option<i32>) -> String {
    use crate::i18n::{tr, trf};
    match (code as u32, status) {
        (0x8004_5509, _) => tr("voice.privacy").into(),
        (0x8007_0005, _) => tr("voice.mic_blocked").into(),
        (0x8004_503A, _) | (0x8000_4005, Some(10)) | (_, Some(10)) => tr("voice.no_mic").into(),
        (_, Some(1)) | (0x8004_5500..=0x8004_5508, _) => tr("voice.no_language").into(),
        (_, Some(9)) => tr("voice.no_network").into(),
        _ => trf("voice.failed", &[("code", &format!("{:#010x}", code as u32))]),
    }
}

fn err_msg(e: &windows::core::Error) -> String {
    explain(e.code().0, None)
}

fn pick_language() -> windows::core::Result<Language> {
    let list = SpeechRecognizer::SupportedTopicLanguages()?;
    let mut tags = Vec::new();
    for i in 0..list.Size()? {
        tags.push(list.GetAt(i)?.LanguageTag()?.to_string());
    }
    let system = SpeechRecognizer::SystemSpeechLanguage().and_then(|l| l.LanguageTag()).map(|t| t.to_string()).unwrap_or_default();
    let (want, main) = match crate::i18n::current() {
        crate::i18n::Lang::Es => ("es", "es-ES"),
        crate::i18n::Lang::En => ("en", "en-US"),
        crate::i18n::Lang::PtBr => ("pt", "pt-BR"),
        crate::i18n::Lang::Fr => ("fr", "fr-FR"),
        crate::i18n::Lang::De => ("de", "de-DE"),
        crate::i18n::Lang::It => ("it", "it-IT"),
    };
    match choose_language_for(&tags, &system, want, main) {
        Some(t) => Language::CreateLanguage(&HSTRING::from(t)),
        // Sin ningún idioma de dictado instalado
        None => Err(windows::core::Error::new(windows::core::HRESULT(0x8004_5500u32 as i32), "sin idioma")),
    }
}

fn join_text(done: &str, partial: &str) -> String {
    match (done.trim(), partial.trim()) {
        ("", p) => p.to_string(),
        (d, "") => d.to_string(),
        (d, p) => format!("{d} {p}"),
    }
}

fn session(app: AppHandle, window: String, ready: mpsc::Sender<Result<(), String>>, stop: mpsc::Receiver<()>) {
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let emit = {
        let app = app.clone();
        let window = window.clone();
        move |ev: &str, text: String| {
            let _ = app.emit(ev, VoiceEvent { window: window.clone(), text });
        }
    };
    let setup = || -> Result<(SpeechRecognizer, SpeechContinuousRecognitionSession), String> {
        let rec = SpeechRecognizer::Create(&pick_language().map_err(|e| err_msg(&e))?).map_err(|e| err_msg(&e))?;
        let topic = SpeechRecognitionTopicConstraint::Create(SpeechRecognitionScenario::Dictation, &HSTRING::from("dictation")).map_err(|e| err_msg(&e))?;
        rec.Constraints().and_then(|c| c.Append(&topic)).map_err(|e| err_msg(&e))?;
        let compiled = rec.CompileConstraintsAsync().and_then(|op| op.join()).map_err(|e| err_msg(&e))?;
        let status = compiled.Status().map_err(|e| err_msg(&e))?;
        if status != SpeechRecognitionResultStatus::Success {
            return Err(explain(0, Some(status.0)));
        }
        let s = rec.ContinuousRecognitionSession().map_err(|e| err_msg(&e))?;
        let _ = s.SetAutoStopSilenceTimeout(windows::Foundation::TimeSpan { Duration: 60 * 10_000_000 });
        Ok((rec, s))
    };
    let (rec, sess) = match setup() {
        Ok(v) => v,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };

    let done = Arc::new(Mutex::new(String::new()));
    {
        let done = done.clone();
        let emit = emit.clone();
        let _ = rec.HypothesisGenerated(&TypedEventHandler::new(move |_, args: windows::core::Ref<SpeechRecognitionHypothesisGeneratedEventArgs>| {
            if let Ok(a) = args.ok() {
                let h = a.Hypothesis()?.Text()?.to_string();
                emit("voice-partial", join_text(&done.lock().unwrap(), &h));
            }
            Ok(())
        }));
    }
    {
        let done = done.clone();
        let emit = emit.clone();
        let _ = sess.ResultGenerated(&TypedEventHandler::new(move |_, args: windows::core::Ref<SpeechContinuousRecognitionResultGeneratedEventArgs>| {
            if let Ok(a) = args.ok() {
                let r = a.Result()?;
                if r.Status()? == SpeechRecognitionResultStatus::Success && r.Confidence()? != SpeechRecognitionConfidence::Rejected {
                    let mut d = done.lock().unwrap();
                    *d = join_text(&d, &r.Text()?.to_string());
                    emit("voice-partial", d.clone());
                }
            }
            Ok(())
        }));
    }
    {
        let emit = emit.clone();
        let _ = sess.Completed(&TypedEventHandler::new(move |_, args: windows::core::Ref<SpeechContinuousRecognitionCompletedEventArgs>| {
            if let Ok(a) = args.ok() {
                let st = a.Status()?;
                if st != SpeechRecognitionResultStatus::Success && st != SpeechRecognitionResultStatus::UserCanceled && st != SpeechRecognitionResultStatus::TimeoutExceeded {
                    emit("voice-error", explain(0, Some(st.0)));
                }
            }
            Ok(())
        }));
    }

    if let Err(e) = sess.StartAsync().and_then(|op| op.join()) {
        let _ = ready.send(Err(err_msg(&e)));
        return;
    }
    let _ = ready.send(Ok(()));
    // Escuchar hasta soltar (como mucho un minuto)
    let _ = stop.recv_timeout(Duration::from_secs(60));
    // Parar entrega los últimos resultados antes de terminar
    let _ = sess.StopAsync().and_then(|op| op.join());
    std::thread::sleep(Duration::from_millis(150));
    let text = done.lock().unwrap().clone();
    emit("voice-final", text);
    drop(rec);
}

/// Empieza a escuchar. Termina con `voice_stop` (o al minuto).
#[tauri::command(async)]
pub fn voice_start(app: AppHandle, window: tauri::Window) -> Result<(), String> {
    let (stop_tx, stop_rx) = mpsc::channel();
    // Si había otra escucha, se para; si sueltan antes de arrancar, el aviso queda en la cola
    if let Some(old) = STOP.lock().unwrap().replace(stop_tx) {
        let _ = old.send(());
    }
    let (ready_tx, ready_rx) = mpsc::channel();
    let label = window.label().to_string();
    std::thread::spawn(move || session(app, label, ready_tx, stop_rx));
    match ready_rx.recv_timeout(Duration::from_secs(15)) {
        Ok(r) => r,
        Err(_) => Err(crate::i18n::tr("voice.timeout").into()),
    }
}

#[tauri::command(async)]
pub fn voice_stop() {
    if let Some(tx) = STOP.lock().unwrap().take() {
        let _ = tx.send(());
    }
}

/// Abre la página de Configuración que arregla el problema
#[tauri::command(async)]
pub fn voice_open_settings(page: String) -> Result<(), String> {
    let uri = match page.as_str() {
        "speech" => "ms-settings:speech",
        "privacy-microphone" => "ms-settings:privacy-microphone",
        _ => "ms-settings:privacy-speech",
    };
    std::process::Command::new("explorer").arg(uri).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idioma_preferido() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(choose_language(&s(&["en-US", "es-MX", "es-ES"]), "es-MX").as_deref(), Some("es-MX"));
        assert_eq!(choose_language(&s(&["en-US", "es-ES"]), "en-US").as_deref(), Some("es-ES"));
        assert_eq!(choose_language(&s(&["en-US", "es-US"]), "en-US").as_deref(), Some("es-US"));
        assert_eq!(choose_language(&s(&["en-US", "fr-FR"]), "fr-FR").as_deref(), Some("fr-FR"));
        assert_eq!(choose_language(&[], "es-ES"), None);
    }

    #[test]
    fn errores_claros() {
        assert!(explain(0x8004_5509u32 as i32, None).contains("Reconocimiento de voz en línea"));
        assert!(explain(0, Some(1)).contains("paquete de voz"));
        assert!(explain(0x8007_0005u32 as i32, None).contains("Micrófono"));
        assert!(explain(0, Some(9)).contains("internet"));
        assert_eq!(join_text("hola", " qué tal "), "hola qué tal");
        assert_eq!(join_text("", "hola"), "hola");
    }

    /// Solo en un PC real: comprueba que Windows da la lista de idiomas de dictado
    #[test]
    #[ignore]
    fn idiomas_del_sistema() {
        let l = pick_language().map(|l| l.LanguageTag().unwrap().to_string());
        println!("{l:?}");
    }
}
