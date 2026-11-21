//! El tiempo del salvapantallas, con Open-Meteo (gratis y sin clave).
//! · Buscar ciudad: geocoding-api.open-meteo.com
//! · Tiempo actual: api.open-meteo.com (se guarda 20 minutos)
//! · "Ubicación aproximada": solo si la persona la elige, con un servicio por IP (ipwho.is o ipapi.co)

use crate::i18n::tr;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const CACHE: Duration = Duration::from_secs(20 * 60);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub name: String,
    /// Región y país, para distinguir ciudades con el mismo nombre
    pub detail: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    pub place: String,
    pub temperature: f64,
    pub code: u32,
    pub is_day: bool,
    /// Texto corto en español ("Nublado")
    pub label: String,
    /// Nombre del icono de Phosphor ("Cloud", "Sun"…)
    pub icon: String,
}

static CACHED: Mutex<Option<(String, Instant, Weather)>> = Mutex::new(None);
static AUTO_PLACE: Mutex<Option<Place>> = Mutex::new(None);

/// Código WMO → (texto, icono de Phosphor)
pub fn describe(code: u32, is_day: bool) -> (&'static str, &'static str) {
    let (key, icon) = match code {
        0 => ("weather.clear", if is_day { "Sun" } else { "Moon" }),
        1 => ("weather.mostly_clear", if is_day { "Sun" } else { "Moon" }),
        2 => ("weather.partly_cloudy", if is_day { "CloudSun" } else { "CloudMoon" }),
        3 => ("weather.cloudy", "Cloud"),
        45 | 48 => ("weather.fog", "CloudFog"),
        51 | 53 | 55 => ("weather.drizzle", "CloudRain"),
        56 | 57 => ("weather.freezing_drizzle", "CloudRain"),
        61 => ("weather.light_rain", "CloudRain"),
        63 => ("weather.rain", "CloudRain"),
        65 => ("weather.heavy_rain", "CloudRain"),
        66 | 67 => ("weather.freezing_rain", "CloudRain"),
        71 => ("weather.light_snow", "CloudSnow"),
        73 => ("weather.snow", "CloudSnow"),
        75 => ("weather.heavy_snow", "CloudSnow"),
        77 => ("weather.snow_grains", "CloudSnow"),
        80 | 81 => ("weather.showers", "CloudRain"),
        82 => ("weather.heavy_showers", "CloudRain"),
        85 | 86 => ("weather.snow_showers", "CloudSnow"),
        95 => ("weather.storm", "CloudLightning"),
        96 | 99 => ("weather.storm_hail", "CloudLightning"),
        _ => ("weather.variable", "Cloud"),
    };
    (tr(key), icon)
}

pub fn geocode_url(q: &str) -> String {
    let lang = crate::i18n::code().split('-').next().unwrap_or("es");
    reqwest::Url::parse_with_params(
        "https://geocoding-api.open-meteo.com/v1/search",
        &[("name", q.trim()), ("count", "5"), ("language", lang), ("format", "json")],
    )
    .map(|u| u.to_string())
    .unwrap_or_default()
}

pub fn forecast_url(lat: f64, lon: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}&current=temperature_2m,weather_code,is_day&timezone=auto"
    )
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Consola/1.0")
        .build()
        .map_err(|e| e.to_string())
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let r = client()?.get(url).send().map_err(|_| tr("common.no_internet").to_string())?;
    let v: serde_json::Value = r.json().map_err(|_| tr("weather.bad_reply").to_string())?;
    Ok(v)
}

/// Respuesta del buscador de ciudades → lugares
pub fn parse_places(v: &serde_json::Value) -> Vec<Place> {
    let Some(list) = v.get("results").and_then(|r| r.as_array()) else { return vec![] };
    list.iter()
        .filter_map(|r| {
            let name = r.get("name")?.as_str()?.to_string();
            let detail = ["admin1", "country"]
                .iter()
                .filter_map(|k| r.get(*k).and_then(|x| x.as_str()))
                .filter(|s| *s != name)
                .collect::<Vec<_>>()
                .join(", ");
            Some(Place { name, detail, latitude: r.get("latitude")?.as_f64()?, longitude: r.get("longitude")?.as_f64()? })
        })
        .collect()
}

pub fn parse_current(v: &serde_json::Value, place: &str) -> Result<Weather, String> {
    if v.get("error").and_then(|e| e.as_bool()) == Some(true) {
        return Err(tr("weather.down").into());
    }
    let c = v.get("current").ok_or(tr("weather.no_data"))?;
    let temperature = c.get("temperature_2m").and_then(|x| x.as_f64()).ok_or(tr("weather.no_temp"))?;
    let code = c.get("weather_code").and_then(|x| x.as_u64()).unwrap_or(3) as u32;
    let is_day = c.get("is_day").and_then(|x| x.as_u64()).unwrap_or(1) == 1;
    let (label, icon) = describe(code, is_day);
    Ok(Weather { place: place.to_string(), temperature, code, is_day, label: label.into(), icon: icon.into() })
}

/// Ubicación aproximada por la IP: ipwho.is y, si falla, ipapi.co
fn locate_by_ip() -> Result<Place, String> {
    if let Some(p) = AUTO_PLACE.lock().unwrap().clone() {
        return Ok(p);
    }
    let from = |v: &serde_json::Value, country_key: &str| -> Option<Place> {
        Some(Place {
            name: v.get("city")?.as_str()?.to_string(),
            detail: [v.get("region").and_then(|x| x.as_str()), v.get(country_key).and_then(|x| x.as_str())]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", "),
            latitude: v.get("latitude")?.as_f64()?,
            longitude: v.get("longitude")?.as_f64()?,
        })
    };
    let place = get_json("https://ipwho.is/")
        .ok()
        .filter(|v| v.get("success").and_then(|s| s.as_bool()) != Some(false))
        .and_then(|v| from(&v, "country"))
        .or_else(|| get_json("https://ipapi.co/json/").ok().and_then(|v| from(&v, "country_name")))
        .ok_or(tr("weather.no_location"))?;
    *AUTO_PLACE.lock().unwrap() = Some(place.clone());
    Ok(place)
}

/// Lugar elegido en los ajustes (o el aproximado)
fn current_place() -> Result<Place, String> {
    let s = crate::idle::settings();
    if s.location_mode == "auto" {
        locate_by_ip()
    } else {
        s.place.ok_or_else(|| tr("weather.pick_city").into())
    }
}

#[tauri::command(async)]
pub fn weather_search(q: String) -> Result<Vec<Place>, String> {
    if q.trim().chars().count() < 2 {
        return Ok(vec![]);
    }
    Ok(parse_places(&get_json(&geocode_url(&q))?))
}

/// Tiempo de ahora en el lugar guardado (20 minutos de caché)
#[tauri::command(async)]
pub fn weather_now() -> Result<Weather, String> {
    let place = current_place()?;
    let key = format!("{:.3},{:.3}", place.latitude, place.longitude);
    if let Some((k, at, w)) = CACHED.lock().unwrap().as_ref() {
        if *k == key && at.elapsed() < CACHE {
            // El texto se rehace por si ha cambiado el idioma
            let mut w = w.clone();
            w.label = describe(w.code, w.is_day).0.into();
            return Ok(w);
        }
    }
    let w = parse_current(&get_json(&forecast_url(place.latitude, place.longitude))?, &place.name)?;
    *CACHED.lock().unwrap() = Some((key, Instant::now(), w.clone()));
    Ok(w)
}

/// Lugar aproximado (para enseñarlo en los ajustes al elegir esa opción)
#[tauri::command(async)]
pub fn weather_locate() -> Result<Place, String> {
    locate_by_ip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigos_wmo() {
        assert_eq!(describe(0, true), ("Despejado", "Sun"));
        assert_eq!(describe(0, false), ("Despejado", "Moon"));
        assert_eq!(describe(2, false).1, "CloudMoon");
        assert_eq!(describe(3, true).1, "Cloud");
        assert_eq!(describe(45, true).1, "CloudFog");
        assert_eq!(describe(63, true), ("Lluvia", "CloudRain"));
        assert_eq!(describe(75, true).1, "CloudSnow");
        assert_eq!(describe(95, true).1, "CloudLightning");
        assert_eq!(describe(1234, true).0, "Tiempo variable");
    }

    #[test]
    fn direcciones() {
        assert_eq!(
            geocode_url("San Sebastián de los Reyes"),
            "https://geocoding-api.open-meteo.com/v1/search?name=San+Sebasti%C3%A1n+de+los+Reyes&count=5&language=es&format=json"
        );
        assert_eq!(
            forecast_url(40.4165, -3.70256),
            "https://api.open-meteo.com/v1/forecast?latitude=40.4165&longitude=-3.7026&current=temperature_2m,weather_code,is_day&timezone=auto"
        );
    }

    #[test]
    fn lee_respuestas() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"results":[{"name":"Madrid","latitude":40.4,"longitude":-3.7,"admin1":"Comunidad de Madrid","country":"España"}]}"#,
        )
        .unwrap();
        let p = parse_places(&v);
        assert_eq!(p[0].detail, "Comunidad de Madrid, España");
        let c: serde_json::Value =
            serde_json::from_str(r#"{"current":{"temperature_2m":12.5,"weather_code":61,"is_day":0}}"#).unwrap();
        let w = parse_current(&c, "Madrid").unwrap();
        assert_eq!((w.label.as_str(), w.icon.as_str(), w.is_day), ("Lluvia débil", "CloudRain", false));
        let e: serde_json::Value = serde_json::from_str(r#"{"error":true,"reason":"limit"}"#).unwrap();
        assert!(parse_current(&e, "x").is_err());
    }

    /// Llamada real (una vez, a mano): cargo test --lib weather -- --ignored
    #[test]
    #[ignore]
    fn madrid_de_verdad() {
        let places = weather_search("Madrid".into()).unwrap();
        assert!(!places.is_empty());
        let p = &places[0];
        let v = get_json(&forecast_url(p.latitude, p.longitude)).unwrap();
        println!("{v}");
        let w = parse_current(&v, &p.name).unwrap();
        println!("{w:?}");
    }
}
