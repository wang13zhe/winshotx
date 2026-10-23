use std::path::Path;
use std::process::Command;

use crate::error::{AppError, Result};

#[cfg(windows)]
const NO_WINDOW: u32 = 0x0800_0000; // CREATE_NO_WINDOW: nada de consolas parpadeando

fn command() -> Command {
    let mut cmd = Command::new("ffmpeg");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(NO_WINDOW);
    }
    cmd
}

/// FFmpeg nunca se empaqueta: solo se usa si el usuario ya lo tiene en el PATH.
pub fn available() -> bool {
    command()
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run(args: &[String]) -> Result<()> {
    let output = command()
        .args(args)
        .output()
        .map_err(|e| AppError::Msg(format!("no se ha podido ejecutar ffmpeg: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(4).collect::<Vec<_>>().join(" | ");
        return Err(AppError::Msg(format!("ffmpeg ha fallado: {tail}")));
    }
    Ok(())
}

/// Ruta de maxima calidad: paleta generada por el propio contenido del clip.
pub fn gif_from_video(
    source: &Path,
    destination: &Path,
    fps: u32,
    width: u32,
    quality: u8,
) -> Result<()> {
    let colors = (32 + (quality.clamp(10, 100) as u32 * 224) / 100).min(256);
    let dither = if quality >= 60 {
        "sierra2_4a"
    } else {
        "bayer:bayer_scale=3"
    };
    let filter = format!(
        "fps={fps},scale={width}:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors={colors}:stats_mode=diff[p];[s1][p]paletteuse=dither={dither}:diff_mode=rectangle"
    );
    run(&[
        "-y".into(),
        "-i".into(),
        source.to_string_lossy().to_string(),
        "-filter_complex".into(),
        filter,
        "-loop".into(),
        "0".into(),
        destination.to_string_lossy().to_string(),
    ])
}

pub fn mp4_from_video(
    source: &Path,
    destination: &Path,
    fps: u32,
    width: u32,
    height: u32,
    quality: u8,
) -> Result<()> {
    // CRF invertido: mas calidad pedida, menos compresion con perdida.
    let crf = 34 - (quality.clamp(10, 100) as u32 * 20) / 100;
    let filter = format!("fps={fps},scale={width}:{height}:flags=lanczos");
    run(&[
        "-y".into(),
        "-i".into(),
        source.to_string_lossy().to_string(),
        "-vf".into(),
        filter,
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-crf".into(),
        crf.to_string(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-movflags".into(),
        "+faststart".into(),
        destination.to_string_lossy().to_string(),
    ])
}

/// Cuanto dura el video, en segundos. `None` si no se puede saber.
///
/// Se pregunta a ffmpeg y no se calcula con los fotogramas porque lo que se va a comprimir
/// es un archivo ya escrito, y lo que manda es lo que dura ESE archivo.
pub fn duracion_segundos(ruta: &Path) -> Option<f64> {
    // ffmpeg no tiene un "dime la duracion" limpio como ffprobe: se lee de su informe, que
    // lo imprime al abrir el archivo y sale por stderr. Sin ffprobe instalado, es lo que hay.
    let salida = command()
        .args(["-hide_banner", "-i", &ruta.to_string_lossy()])
        .output()
        .ok()?;
    let informe = String::from_utf8_lossy(&salida.stderr);
    for linea in informe.lines() {
        let Some(pos) = linea.find("Duration:") else { continue };
        let trozo = linea[pos + 9..].trim().split_whitespace().next()?;
        let partes: Vec<&str> = trozo.trim_end_matches(',').split(':').collect();
        if partes.len() != 3 {
            continue;
        }
        let h: f64 = partes[0].parse().ok()?;
        let m: f64 = partes[1].parse().ok()?;
        let s: f64 = partes[2].parse().ok()?;
        let total = h * 3600.0 + m * 60.0 + s;
        if total > 0.0 {
            return Some(total);
        }
    }
    None
}

/// El bitrate de video que hace que un clip de `segundos` ocupe `megas`, descontando el
/// sonido y dejando margen para el contenedor.
///
/// Va aparte y con su prueba porque es la unica cuenta de todo esto que se puede
/// equivocar en silencio: un bitrate mal calculado no falla, da un archivo que no cabe.
fn bitrate_para_caber_en(megas: f64, segundos: f64, audio_kbps: u32) -> u32 {
    if segundos <= 0.0 {
        return 64;
    }
    // 8 bits por byte, y el margen del 8 % para el contenedor y los fotogramas clave.
    let total_kbps = (megas * 8.0 * 1024.0 / segundos * 0.92).max(1.0);
    (total_kbps - f64::from(audio_kbps)).max(64.0).round() as u32
}

/// Vuelve a comprimir un video ya escrito para que quepa en `megas`, o al bitrate que se
/// pida. Es lo que hace falta para mandar una grabacion por un chat o subirla a un sitio
/// con tope: la exportacion normal busca calidad, esta busca tamanno.
///
/// El bitrate se calcula del objetivo y de lo que dura el video, descontando el sonido: si
/// se pidieran seis megas y se dejara el audio a lo que venga, la cuenta saldria corta y el
/// archivo se pasaria igual. Se deja un margen del 8 % porque el contenedor y los
/// fotogramas clave no entran en la cuenta.
///
/// `None` en `megas` significa «usa `bitrate_kbps` tal cual», que es lo que quiere quien ya
/// sabe lo que quiere.
pub fn comprimir(
    source: &Path,
    destination: &Path,
    megas: Option<f64>,
    bitrate_kbps: Option<u32>,
    audio_kbps: u32,
) -> Result<()> {
    let kb = match (megas, bitrate_kbps) {
        (Some(m), _) if m > 0.0 => {
            let segundos = duracion_segundos(source).ok_or_else(|| {
                AppError::Msg("no se ha podido saber cuánto dura el vídeo para comprimirlo".into())
            })?;
            bitrate_para_caber_en(m, segundos, audio_kbps)
        }
        (_, Some(k)) if k > 0 => k,
        _ => {
            return Err(AppError::Msg(
                "hay que decir cuánto tiene que ocupar o a qué bitrate".into(),
            ))
        }
    };

    run(&[
        "-y".into(),
        "-i".into(),
        source.to_string_lossy().to_string(),
        // Dos pasadas serian mejores, pero cuestan el doble y esto es para un archivo que
        // se va a ver en un chat. Una pasada con `-maxrate` y `-bufsize` acota el pico,
        // que es lo que hace que un video "de 10 MB" acabe en 14.
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-b:v".into(),
        format!("{kb}k"),
        "-maxrate".into(),
        format!("{kb}k"),
        "-bufsize".into(),
        format!("{}k", kb * 2),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        format!("{audio_kbps}k"),
        "-movflags".into(),
        "+faststart".into(),
        destination.to_string_lossy().to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un minuto de video en 10 MB son unos 1.300 kbps en total. Con 128 de sonido quedan
    /// unos 1.200 de imagen, y con el margen del 8 % algo menos.
    #[test]
    fn el_bitrate_sale_de_lo_que_tiene_que_ocupar() {
        let kb = bitrate_para_caber_en(10.0, 60.0, 128);
        // 10 MB * 8 * 1024 / 60 s = 1.365 kbps, menos el 8 % = 1.256, menos 128 de sonido.
        assert!(
            (1_050..=1_200).contains(&kb),
            "para 10 MB en un minuto salieron {kb} kbps"
        );
    }

    /// La mitad de tiempo, el doble de bitrate PARA LA IMAGEN. El total no se dobla porque
    /// el sonido es fijo: si se doblara, el archivo se pasaria del tope en el clip corto.
    #[test]
    fn la_mitad_de_duracion_da_el_doble_de_bitrate_de_imagen() {
        let audio = 128;
        let largo = bitrate_para_caber_en(10.0, 60.0, audio);
        let corto = bitrate_para_caber_en(10.0, 30.0, audio);
        // Lo que se dobla es el total antes de descontar el sonido.
        let total_largo = largo + audio;
        let total_corto = corto + audio;
        assert!(
            (total_corto as f64 / total_largo as f64 - 2.0).abs() < 0.05,
            "el total tenia que doblarse: {total_largo} y {total_corto}"
        );
        // Y la imagen crece mas que el doble, porque el sonido no crece.
        assert!(
            corto > largo * 2,
            "la imagen del clip corto tiene que pasar del doble: {largo} y {corto}"
        );
    }

    /// Y un objetivo imposible no puede dar un bitrate de cero o negativo: el codificador
    /// no arranca y el usuario ve un fallo sin explicacion.
    #[test]
    fn un_objetivo_imposible_da_el_minimo_y_no_cero() {
        // Un video de una hora en 1 MB no cabe en ningun bitrate util.
        assert_eq!(bitrate_para_caber_en(1.0, 3600.0, 128), 64);
        // Y sin duracion tampoco se puede dividir por cero.
        assert_eq!(bitrate_para_caber_en(10.0, 0.0, 128), 64);
    }

    /// El sonido se descuenta del total, no se suma: si no, el archivo se pasa del tope.
    #[test]
    fn el_sonido_se_descuenta_del_total() {
        let sin_sonido = bitrate_para_caber_en(10.0, 60.0, 0);
        let con_sonido = bitrate_para_caber_en(10.0, 60.0, 192);
        assert_eq!(sin_sonido - con_sonido, 192, "el sonido tiene que salir del total");
    }
}
