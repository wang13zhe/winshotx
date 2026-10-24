//! Coser una captura larga: varias fotos de la misma zona mientras el contenido se
//! desplaza, pegadas en una sola imagen.
//!
//! El problema no es pegar: es **saber cuanto se ha desplazado**. Nadie dice «he bajado 40
//! pixeles»; hay que deducirlo comparando la foto nueva con la anterior. Si se pega de mas,
//! el resultado repite una franja; si se pega de menos, se come contenido. Las dos cosas se
//! ven, y las dos son peores que no hacer la captura larga.
//!
//! Aqui solo esta ese calculo, que es codigo puro y se prueba sin capturar nada: cuantas
//! filas de abajo de la foto nueva son contenido que no estaba en la anterior.
//!
//! **Se compara por huellas de fila y no pixel a pixel.** Buscar la franja de abajo en cada
//! desplazamiento posible, comparando filas enteras, son cientos de millones de
//! comparaciones por foto; con una huella por fila cada candidato se reduce a comparar
//! enteros. Y no se pierde nada: dos filas de una captura de pantalla o son iguales o se
//! diferencian en la mayoria de sus pixeles.

use image::RgbaImage;

/// De cuantos pixeles se coge la huella de una fila. Cada cuatro es de sobra para una
/// pantalla, y baja el coste por fila a la cuarta parte.
const PASO: usize = 4;

/// La huella de una fila: los canales de color de uno de cada `PASO` pixeles, apretados en
/// un numero. FNV-1a, que es una multiplicacion y un xor por byte y no necesita tabla.
fn huella(fila: &[u8], ancho: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut x = 0;
    while x < ancho {
        let p = x * 4;
        // Los tres canales de color, no el alfa: el alfa de una captura de pantalla es 255
        // en todas partes y no distingue nada.
        for canal in 0..3 {
            h ^= u64::from(fila[p + canal]);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        x += PASO;
    }
    h
}

/// La huella de cada fila de una imagen.
fn huellas(imagen: &RgbaImage) -> Vec<u64> {
    let ancho = imagen.width() as usize;
    let fila = ancho * 4;
    let datos = imagen.as_raw();
    (0..imagen.height() as usize)
        .map(|y| huella(&datos[y * fila..(y + 1) * fila], ancho))
        .collect()
}

/// Cuantas filas de abajo de `nueva` son contenido que no estaba en `anterior`.
///
/// `franja` es cuantas filas de `anterior` se buscan dentro de `nueva` (la tira que se
/// espera volver a encontrar), y `minimo` es que fraccion de esas filas tiene que coincidir
/// para dar el hallazgo por bueno. Sin ese minimo, dos zonas lisas cualesquiera (un fondo
/// blanco, un bloque de color) coinciden en cualquier sitio y el resultado sale cortado.
///
/// Devuelve cero cuando no hay nada que añadir: la imagen no se ha movido, no se ha podido
/// encontrar la franja, o las dos fotos no son de lo mismo.
pub fn filas_nuevas(anterior: &RgbaImage, nueva: &RgbaImage, franja: u32, minimo: f64) -> u32 {
    if anterior.width() != nueva.width() || anterior.width() == 0 {
        return 0;
    }
    let alto_previo = anterior.height();
    let alto_nuevo = nueva.height();
    // Con menos filas que esto no hay franja que buscar: dos filas coinciden por casualidad.
    let franja = franja.min(alto_previo).min(alto_nuevo);
    if franja < 8 || alto_nuevo < franja || alto_previo < franja {
        return 0;
    }

    let h_anterior = huellas(anterior);
    let h_nueva = huellas(nueva);
    let patron = &h_anterior[(alto_previo - franja) as usize..];

    let exigidas = ((franja as f64) * minimo.clamp(0.0, 1.0)).ceil() as u32;
    let ultimo = alto_nuevo - franja;
    let mut elegido: Option<u32> = None;
    let mut k = 0;
    while k <= ultimo {
        let mut coinciden = 0u32;
        for (i, p) in patron.iter().enumerate() {
            if h_nueva[(k as usize) + i] == *p {
                coinciden += 1;
                if coinciden >= exigidas {
                    break;
                }
            }
        }
        if coinciden >= exigidas {
            // Se prefiere el desplazamiento MAS PEQUEÑO de los que encajan, que es el `k`
            // mas grande. Dos sitios que encajan suelen ser contenido repetido (una lista,
            // una tabla): quedarse con el mayor se saltaria lo de en medio.
            elegido = Some(k);
        }
        k += 1;
    }

    let Some(k) = elegido else { return 0 };
    // Lo que queda por debajo de la franja encontrada es lo nuevo. Si la franja acaba justo
    // al final, no hay nada nuevo: la pantalla no se movio.
    alto_nuevo.saturating_sub(k + franja)
}

/// Pega `nueva` debajo de `cosida`, saltandose las filas que ya estaban.
///
/// Devuelve cuantas filas se han añadido de verdad.
pub fn coser(cosida: &mut RgbaImage, nueva: &RgbaImage, filas: u32) -> u32 {
    if filas == 0 || nueva.height() < filas {
        return 0;
    }
    let ancho = cosida.width();
    if nueva.width() != ancho {
        return 0;
    }
    let alto_previo = cosida.height();
    let alto_final = alto_previo + filas;
    let fila = ancho as usize * 4;
    let mut lienzo = RgbaImage::new(ancho, alto_final);
    let previo = cosida.as_raw();
    let nuevo = nueva.as_raw();
    let destino = lienzo.as_mut();
    // La parte de arriba es lo que ya habia. Se copia entera y no se toca: es la que el
    // usuario ya vio bien en la foto anterior.
    let corte = alto_previo as usize * fila;
    destino[..corte].copy_from_slice(&previo[..corte]);
    // Y lo nuevo, las ultimas `filas` de la foto. Una sola copia de memoria, que a 1080p son
    // ocho megabytes y no conviene pasarlos por una imagen intermedia.
    let desde = (nueva.height() - filas) as usize * fila;
    let cuantas = filas as usize * fila;
    destino[corte..corte + cuantas].copy_from_slice(&nuevo[desde..desde + cuantas]);
    *cosida = lienzo;
    filas
}

// ---------------------------------------------------------------------------
// La sesion en curso: todo lo que hace falta acordarse entre foto y foto
// ---------------------------------------------------------------------------

/// Cuanta parte de abajo de la foto se busca en la anterior, en tanto por uno.
///
/// Un 60 % de una ventana de 1080 lineas son 648 lineas: cualquier rueda de raton baja
/// bastante menos, asi que la ventana siempre tiene por donde moverse. Si se pusiera muy
/// pequena, un salto grande se saldria de la franja y no se podria casar.
pub const FRANJA: f64 = 0.6;

/// Cuanto de la franja tiene que encajar para darla por buena, en tanto por uno.
///
/// El 70 % deja sitio para el raton encima, para un menu que se cierra y para una
/// animacion que va por su cuenta. Pero si se bajara mas, dos zonas vacias de la misma
/// pagina (dos listas iguales) casarian entre si y la imagen saldria con un trozo
/// repetido en medio.
pub const MINIMO: f64 = 0.7;

/// Una captura larga a medias.
///
/// `anterior` es privada a proposito: quien llama solo da fotos nuevas y lee el resultado.
/// Dejarla visible invita a usarla como si fuera el estado, y no lo es: cambia en cada
/// `paso` y antes del primer paso no existe nada.
pub struct EnCurso {
    /// La zona elegida, en coordenadas del escritorio. Se guarda para poder seguir
    /// fotografiando exactamente el mismo rectangulo mientras el usuario baja.
    pub region: crate::capture::Rect,
    cosida: image::RgbaImage,
    anterior: image::RgbaImage,
    /// Filas que se han añadido por encima de la primera foto.
    anadidas: u32,
    /// Cuantas fotos se han cosido, contando la primera.
    capturas: u32,
}

impl EnCurso {
    /// Abre la sesion con la primera foto.
    pub fn nueva(region: crate::capture::Rect, primera: image::RgbaImage) -> Self {
        Self {
            region,
            anterior: primera.clone(),
            cosida: primera,
            anadidas: 0,
            capturas: 1,
        }
    }

    /// Cose una foto mas. Devuelve las filas nuevas que ha traido.
    ///
    /// Cero no es un error: es que la pantalla no se movio, o se movio mas de lo que la
    /// franja alcanza a ver. Las dos cosas le pasan a alguien que baja la rueda sin parar.
    pub fn paso(&mut self, nueva: image::RgbaImage) -> u32 {
        if nueva.width() != self.anterior.width() || nueva.height() != self.anterior.height() {
            // La ventana se ha redimensionado a mitad del barrido: las huellas de fila ya
            // no significan nada y compararlas daria un desplazamiento inventado. Se
            // descarta la foto y se espera a la siguiente; si el tamaño se ha quedado asi,
            // el resto del barrido no aporta nada y se acaba con lo cosido hasta aqui.
            return 0;
        }
        let franja = ((nueva.height() as f64 * FRANJA).round() as u32).max(8);
        let anadidas = filas_nuevas(&self.anterior, &nueva, franja, MINIMO);
        if anadidas == 0 {
            self.anterior = nueva;
            return 0;
        }
        let anadidas = coser(&mut self.cosida, &nueva, anadidas);
        self.anterior = nueva;
        self.capturas += 1;
        self.anadidas += anadidas;
        anadidas
    }

    /// La imagen cosida hasta ahora.
    pub fn imagen(&self) -> &image::RgbaImage {
        &self.cosida
    }

    /// Filas añadidas por encima de la primera foto.
    pub fn anadidas(&self) -> u32 {
        self.anadidas
    }

    /// Cuantas fotos se han cosido, contando la primera.
    pub fn capturas(&self) -> u32 {
        self.capturas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// Un documento alto de verdad: filas de color distinto y con marcas, para que dos
    /// trozos distintos no se parezcan. Un fondo liso haria coincidir cualquier cosa.
    fn documento(ancho: u32, alto: u32) -> RgbaImage {
        RgbaImage::from_fn(ancho, alto, |x, y| {
            let base = (y % 251) as u8;
            let marca = if (x + y) % 37 == 0 { 200 } else { 0 };
            Rgba([base, base.wrapping_add(marca), (x % 199) as u8, 255])
        })
    }

    /// La vista de una ventana del documento: lo que se ve con el scroll en `desde`.
    fn ventana(doc: &RgbaImage, desde: u32, alto: u32) -> RgbaImage {
        image::imageops::crop_imm(doc, 0, desde, doc.width(), alto).to_image()
    }

    #[test]
    fn una_pantalla_que_no_se_mueve_no_aporta_nada() {
        let doc = documento(120, 400);
        let a = ventana(&doc, 0, 100);
        let b = ventana(&doc, 0, 100);
        assert_eq!(filas_nuevas(&a, &b, 60, 0.7), 0, "sin desplazamiento no hay nada nuevo");
    }

    #[test]
    fn un_desplazamiento_conocido_aporta_esas_filas() {
        let doc = documento(120, 400);
        let a = ventana(&doc, 0, 100);
        // Se ha bajado 40 pixeles: la nueva foto enseña del 40 al 140.
        let b = ventana(&doc, 40, 100);
        assert_eq!(
            filas_nuevas(&a, &b, 60, 0.7),
            40,
            "bajando cuarenta pixeles tienen que entrar cuarenta filas"
        );
    }

    /// Y con desplazamientos de distintos tamaños, siempre los que tocan.
    ///
    /// El tope esta en `alto - franja`: mas alla de eso la tira que se busca ya se ha salido
    /// por arriba y no hay nada con que casar. Por eso la franja de la captura larga de
    /// verdad es ancha (sesenta o setenta por ciento del alto): cuanto mas ancha, mas puede
    /// saltar quien maneja la rueda sin que se pierda el hilo.
    #[test]
    fn cada_desplazamiento_aporta_lo_suyo() {
        let doc = documento(160, 800);
        let (alto, franja) = (200u32, 80u32);
        let a = ventana(&doc, 0, alto);
        for salto in [1u32, 7, 33, 100, alto - franja] {
            let b = ventana(&doc, salto, alto);
            assert_eq!(
                filas_nuevas(&a, &b, franja, 0.7),
                salto,
                "un salto de {salto} pixeles tiene que dar {salto} filas"
            );
        }
    }

    /// Y un salto tan grande que la tira ya se ha salido por arriba no se puede casar: se
    /// devuelve cero en vez de inventarse un empalme. Es la razon de que la captura larga
    /// pida ir despacio: prefijar de mas es peor que no coser.
    #[test]
    fn un_salto_mayor_que_la_franja_no_se_puede_casar() {
        let doc = documento(160, 900);
        let (alto, franja) = (200u32, 80u32);
        let a = ventana(&doc, 0, alto);
        // 180 > 200 - 80: la tira de abajo de `a` (doc 120..200) ya no esta en `b`
        // (doc 180..380) y no hay nada que buscar.
        let b = ventana(&doc, 180, alto);
        assert_eq!(filas_nuevas(&a, &b, franja, 0.7), 0);
    }

    /// Dos fotos que no tienen nada que ver no se cosen: mas vale no hacer la captura
    /// larga que pegarla mal.
    #[test]
    fn dos_cosas_distintas_no_se_cosen() {
        let ancho = 120;
        let liso = RgbaImage::from_pixel(ancho, 100, Rgba([255, 255, 255, 255]));
        let ruido = RgbaImage::from_fn(ancho, 100, |x, y| {
            Rgba([(x * 7 + y * 13) as u8, (x * 3) as u8, (y * 5) as u8, 255])
        });
        assert_eq!(filas_nuevas(&liso, &ruido, 50, 0.7), 0);
    }

    /// Coser de verdad y comprobar que la imagen sale entera y en orden.
    #[test]
    fn lo_cosido_junta_las_dos_fotos_sin_repetir_ni_perder() {
        let doc = documento(120, 400);
        let a = ventana(&doc, 0, 100);
        let mut cosida = a.clone();
        // Tres desplazamientos seguidos de 40, como haria alguien bajando la rueda.
        let mut total = 0;
        for paso in 1..=3u32 {
            let b = ventana(&doc, paso * 40, 100);
            let filas = filas_nuevas(&ventana(&doc, (paso - 1) * 40, 100), &b, 60, 0.7);
            total += coser(&mut cosida, &b, filas);
        }
        assert_eq!(total, 120, "tres saltos de cuarenta son ciento veinte filas");
        assert_eq!(cosida.height(), 220, "cien de la primera mas ciento veinte");
        // Y lo cosido es exactamente el documento del 0 al 220, sin repetir nada.
        let esperado = ventana(&doc, 0, 220);
        assert_eq!(cosida.as_raw(), esperado.as_raw(), "la captura larga sale repetida o cortada");
    }

    /// Coser cero filas no puede tocar la imagen.
    #[test]
    fn coser_cero_no_cambia_nada() {
        let doc = documento(120, 400);
        let mut cosida = ventana(&doc, 0, 100);
        let antes = cosida.clone();
        assert_eq!(coser(&mut cosida, &ventana(&doc, 40, 100), 0), 0);
        assert_eq!(cosida.as_raw(), antes.as_raw());
    }

    /// Y un ancho distinto no se cose: la ventana se ha redimensionado a mitad.
    #[test]
    fn un_ancho_distinto_no_se_cose() {
        let doc = documento(120, 400);
        let mut cosida = ventana(&doc, 0, 100);
        let otra = documento(200, 100);
        assert_eq!(coser(&mut cosida, &otra, 30), 0);
        assert_eq!(cosida.height(), 100);
    }

    /// Un rectangulo cualquiera: `EnCurso` solo lo guarda para poder seguir fotografiando
    /// el mismo sitio, asi que aqui da igual cual sea.
    fn zona(ancho: u32, alto: u32) -> crate::capture::Rect {
        crate::capture::Rect {
            x: 10,
            y: 20,
            width: ancho,
            height: alto,
        }
    }

    /// El barrido bueno: se va bajando y la imagen va creciendo con lo nuevo.
    #[test]
    fn un_barrido_largo_va_cosiendo_sin_repetir_filas() {
        let doc = documento(300, 2000);
        let mut sesion = EnCurso::nueva(zona(300, 400), ventana(&doc, 0, 400));
        let mut cosido = 0;
        // Saltos de 40 filas: por debajo del techo de `alto - franja` (400 - 240).
        for i in 1..=6 {
            let anadidas = sesion.paso(ventana(&doc, 40 * i, 400));
            assert!(anadidas > 0, "el salto {i} tenia que aportar filas");
            cosido += anadidas;
            assert_eq!(sesion.imagen().height(), 400 + cosido);
        }
        assert_eq!(cosido, 240, "seis saltos de 40 filas son 240 filas nuevas");
        assert_eq!(sesion.anadidas(), 240);
        assert_eq!(sesion.capturas(), 7);
    }

    /// Una pantalla quieta no estropea nada: no crece, pero tampoco se descarta la sesion.
    #[test]
    fn una_pantalla_quieta_deja_la_sesion_viva() {
        let doc = documento(200, 800);
        let mut sesion = EnCurso::nueva(zona(200, 200), ventana(&doc, 0, 200));
        assert_eq!(sesion.paso(ventana(&doc, 0, 200)), 0);
        assert_eq!(sesion.paso(ventana(&doc, 0, 200)), 0);
        assert_eq!(sesion.imagen().height(), 200, "quieta no crece");
        assert_eq!(sesion.capturas(), 1, "y no cuenta como foto cosida");
        // Y se puede seguir: lo cosido hasta aqui sigue valiendo.
        assert!(sesion.paso(ventana(&doc, 100, 200)) > 0);
    }

    /// Si la ventana cambia de tamaño a mitad, esa foto se descarta en lugar de inventarse
    /// un desplazamiento. La sesion sigue viva por si vuelve al tamaño anterior.
    #[test]
    fn un_cambio_de_tamaño_descarta_la_foto() {
        let doc = documento(200, 800);
        let mut sesion = EnCurso::nueva(zona(200, 200), ventana(&doc, 0, 200));
        let otra = documento(200, 300);
        assert_eq!(sesion.paso(otra), 0);
        assert_eq!(sesion.imagen().height(), 200);
        assert_eq!(sesion.capturas(), 1);
    }

    /// Un salto mayor que la franja no se puede casar. Devuelve cero y no siembra filas
    /// repetidas, que es lo peor que le puede pasar a una captura larga.
    #[test]
    fn un_salto_mayor_que_la_franja_no_cose_nada() {
        let doc = documento(200, 800);
        let mut sesion = EnCurso::nueva(zona(200, 200), ventana(&doc, 0, 200));
        assert_eq!(sesion.paso(ventana(&doc, 400, 200)), 0);
        assert_eq!(sesion.imagen().height(), 200);
    }
}