import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Download, MousePointer2, Type, X } from "lucide-react";
import { copyPinned, pinnedText, savePinned } from "../../lib/ipc";
import { useT } from "../../lib/i18n";

/**
 * Una captura dejada flotando encima de todo, para tenerla delante mientras se trabaja.
 *
 * Nace exactamente encima de donde estaba el recorte y del mismo tamanno, asi que al
 * aparecer no se mueve nada: parece que ese trozo de pantalla se ha quedado quieto. Luego
 * se arrastra con el raton a donde estorbe menos.
 *
 * **Sin barra de titulo, sin cerrar en una esquina y sin nada encima de la imagen.** Los
 * botones solo salen al pasar el raton por encima, y en la esquina de arriba a la
 * derecha, que es donde ya los busca todo el mundo. Una captura anclada que lleva
 * cromo alrededor deja de parecer la pantalla y empieza a parecer un visor de imagenes.
 *
 * **Las acciones y sus teclas son las mismas que en la barra de captura**, porque es la
 * misma imagen: copiar, guardar y leer el texto. Aprender que aqui `T` hace otra cosa
 * seria aprender dos programas.
 */
export function PinWindow({ imagen }: { imagen: string }) {
  const t = useT();
  const [encima, setEncima] = useState(false);
  const [aviso, setAviso] = useState<{ texto: string; malo: boolean } | null>(null);

  /**
   * Cuanto se ve la captura. Uno es opaca y es como nace.
   *
   * Una captura anclada tapa lo que hay debajo, y cuando lo que hay debajo es justo lo que
   * se estaba copiando de ella, no hay forma de mirar los dos a la vez. Bajarla con la
   * rueda es mas rapido que arrastrarla a un lado y volverla a traer, y se recuerda
   * mientras dure la ventana.
   *
   * El minimo no es cero: a cero la ventana sigue ahi, invisible y sin forma de recuperarla
   * salvo cerrandola, que es peor que no poder ver lo de debajo.
   */
  const [opacidad, setOpacidad] = useState(1);

  /**
   * Con esto puesto, los clics atraviesan la captura y llegan a lo que hay debajo.
   *
   * Es lo que hace util una captura anclada mientras se trabaja: tener la referencia
   * delante sin que estorbe al pulsar. Se enciende con un boton y se apaga igual, porque
   * con el puesto la ventana ya no recibe clics y el boton tampoco: la unica salida seria
   * cerrarla desde la bandeja. Por eso el atajo de teclado sigue funcionando, que si llega.
   */
  const [atravesable, setAtravesable] = useState(false);
  const ventana = useRef(getCurrentWindow());

  const cerrar = () => void getCurrentWindow().close();

  /**
   * Un solo sitio para las tres acciones, porque las tres terminan igual: un aviso que se
   * va solo. Una captura anclada esta para mirarla, no para leer mensajes.
   */
  const hacer = async (accion: () => Promise<unknown>, bien: string) => {
    try {
      await accion();
      setAviso({ texto: bien, malo: false });
    } catch (e) {
      // Los errores llegan de Rust escritos en espannol, y su texto es la clave.
      setAviso({ texto: String(e), malo: true });
    }
  };

  const copiar = () => hacer(() => copyPinned(imagen), "Copiada");
  const guardar = () => hacer(() => savePinned(imagen), "Guardada");
  const leer = () => hacer(() => pinnedText(imagen), "Texto copiado");

  /**
   * La rueda baja y sube la opacidad de cinco en cinco por ciento.
   *
   * Con `preventDefault` para que no se vaya a la pagina de debajo, que no existe: sin el,
   * el navegador del webview intenta desplazar algo que no se desplaza y el gesto se
   * pierde a la mitad.
   */
  const alRodar = (e: React.WheelEvent) => {
    e.preventDefault();
    const paso = e.deltaY > 0 ? -0.05 : 0.05;
    setOpacidad((v) => Math.min(1, Math.max(0.15, Math.round((v + paso) * 100) / 100)));
  };

  /**
   * Enciende o apaga que los clics atraviesen la ventana.
   *
   * El permiso se pide a la ventana de Tauri, que es la que lo sabe hacer; aqui solo se
   * lleva la cuenta de si esta puesto, porque la ventana no lo puede preguntar despues.
   */
  const alternarAtravesable = async () => {
    const siguiente = !atravesable;
    try {
      await ventana.current.setIgnoreCursorEvents(siguiente);
      setAtravesable(siguiente);
    } catch (e) {
      setAviso({ texto: String(e), malo: true });
    }
  };

  useEffect(() => {
    if (!aviso) return;
    // El malo dura mas: si algo ha salido mal hay que llegar a leerlo.
    const id = setTimeout(() => setAviso(null), aviso.malo ? 3000 : 1400);
    return () => clearTimeout(id);
  }, [aviso]);

  useEffect(() => {
    // Escape cierra, que es lo que hace todo lo demas de winshotx. Las otras tres son las
    // mismas teclas que en la barra de captura.
    const alPulsar = (e: KeyboardEvent) => {
      const tecla = e.key.toLowerCase();
      if (e.key === "Escape") {
        cerrar();
      } else if ((e.ctrlKey || e.metaKey) && tecla === "c") {
        void copiar();
      } else if ((e.ctrlKey || e.metaKey) && tecla === "s") {
        // Sin esto, el navegador que hay debajo abriria su cuadro de guardar la pagina.
        e.preventDefault();
        void guardar();
      } else if (tecla === "t" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        void leer();
      } else if (tecla === "x" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        // La unica forma de salir de «atravesable» sin cerrar la ventana. Se deja con una
        // tecla suelta y no con Ctrl+X porque con los clics atravesando, el foco del
        // teclado ya no esta aqui y solo llegan los atajos que registra el sistema.
        void alternarAtravesable();
      } else if (tecla === "0" && !e.ctrlKey && !e.metaKey && !e.altKey) {
        setOpacidad(1);
      }
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, [imagen, atravesable]);

  return (
    <div
      data-tauri-drag-region
      onPointerEnter={() => setEncima(true)}
      onPointerLeave={() => setEncima(false)}
      onDoubleClick={cerrar}
      onWheel={alRodar}
      title={t("Arrastra para moverla · rueda para ver a través · Esc para cerrarla")}
      style={{ opacity: opacidad }}
      className="relative h-full w-full cursor-grab overflow-hidden bg-black active:cursor-grabbing"
    >
      <img
        src={convertFileSrc(imagen)}
        alt=""
        draggable={false}
        data-tauri-drag-region
        className="pointer-events-none h-full w-full object-contain"
      />

      <div
        className={`absolute end-1.5 top-1.5 flex gap-1 transition-opacity duration-150 ${
          encima ? "opacity-100" : "pointer-events-none opacity-0"
        }`}
      >
        <Boton icono={Copy} etiqueta={`${t("Copiar")} · Ctrl+C`} onClick={() => void copiar()} />
        <Boton
          icono={Download}
          etiqueta={`${t("Guardar")} · Ctrl+S`}
          onClick={() => void guardar()}
        />
        <Boton icono={Type} etiqueta={`${t("Copiar el texto")} · T`} onClick={() => void leer()} />
        <Boton
          icono={MousePointer2}
          etiqueta={`${t("Dejar pasar los clics")} · X`}
          onClick={() => void alternarAtravesable()}
          activo={atravesable}
        />
        <Boton icono={X} etiqueta={`${t("Cerrar")} · Esc`} onClick={cerrar} peligro />
      </div>

      {aviso && (
        <span
          role="status"
          className={`absolute bottom-2 left-1/2 max-w-[90%] -translate-x-1/2 truncate rounded-md px-2 py-1 text-[11px] font-medium text-white shadow-lg ${
            aviso.malo ? "bg-red-500/90" : "bg-emerald-500/90"
          }`}
        >
          {t(aviso.texto)}
        </span>
      )}
    </div>
  );
}

function Boton({
  icono: Icono,
  etiqueta,
  onClick,
  peligro = false,
  activo = false,
}: {
  icono: typeof Copy;
  etiqueta: string;
  onClick: () => void;
  peligro?: boolean;
  /** Puesto, el boton se pinta encendido: es un interruptor y no una accion de un tiro. */
  activo?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={etiqueta}
      aria-label={etiqueta}
      aria-pressed={activo}
      className={`flex size-6 items-center justify-center rounded-md backdrop-blur-sm transition-colors ${
        peligro
          ? "bg-neutral-900/80 text-neutral-200 hover:bg-red-500 hover:text-white"
          : activo
            ? "bg-blue-500 text-white"
            : "bg-neutral-900/80 text-neutral-200 hover:bg-neutral-700"
      }`}
    >
      <Icono className="size-3.5" />
    </button>
  );
}
