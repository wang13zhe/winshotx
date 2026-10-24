import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Loader2, ScrollText, X } from "lucide-react";
import { GlassPanel } from "../ui/GlassPanel";
import { useT } from "../../lib/i18n";
import { longCaptureStep } from "../../lib/ipc";

interface Props {
  left: number;
  top: number;
  flipped: boolean;
  /** Accion con la que se cerrara el barrido: copiar, guardar o abrir en el editor. */
  action: "copy" | "save" | "save_as" | "edit" | "pin";
  onFinish: () => void;
  onCancel: () => void;
}

/**
 * La barra de la captura larga, que esta en pantalla mientras se barre.
 *
 * Lo importante de esta barra no es lo que ensena sino lo que NO hace: no se cierra. Una
 * captura larga se compone de varias fotos separadas en el tiempo, y entre una y la
 * siguiente el usuario tiene que seguir viendo lo que hay debajo para saber donde esta.
 * Si el overlay se cerrase al empezar, la siguiente foto ya no tendria nada que coser.
 *
 * Las fotos se piden al backend cuando el usuario desplaza algo, no con un temporizador:
 * un reloj sacaria fotos de una pantalla que no se ha movido, y cada foto que no aporta
 * nada es trabajo tirado. Ver `enMarcha`.
 */
export function LongCaptureBar({ left, top, flipped, onFinish, onCancel }: Props) {
  const t = useT();
  const [pasos, setPasos] = useState(0);
  const [alto, setAlto] = useState(0);
  const [anadidas, setAnadidas] = useState(0);
  const [trabajando, setTrabajando] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Dos fotos a la vez no tienen sentido: la segunda coseria contra una `anterior` que
  // la primera todavia no ha escrito. Este candado es el mismo que tiene Rust, pero en
  // el lado que puede disparar mas rapido de lo que el backend responde.
  const enMarcha = useRef(false);

  const darUnPaso = useCallback(async () => {
    if (enMarcha.current) return;
    enMarcha.current = true;
    setTrabajando(true);
    try {
      const p = await longCaptureStep();
      setPasos(p.capturas);
      setAlto(p.alto);
      setAnadidas(p.anadidas);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      enMarcha.current = false;
      setTrabajando(false);
    }
  }, []);

  // La rueda manda una foto. Con `passive: false` se puede dejar de rodar sin mas: si no,
  // el navegador se queda con el evento y la pagina de debajo se mueve sin que aqui se
  // entere nadie, que es justo lo que se quiere evitar.
  useEffect(() => {
    const alRodar = (e: WheelEvent) => {
      e.preventDefault();
      void darUnPaso();
    };
    window.addEventListener("wheel", alRodar, { passive: false });
    return () => window.removeEventListener("wheel", alRodar);
  }, [darUnPaso]);

  // Y el teclado, para quien prefiere no soltar el raton: espacio o enter dan un paso,
  // enter o escape cierran. Se escucha en la ventana y no en un input porque aqui no
  // hay ninguno: todo son botones.
  useEffect(() => {
    const alPulsar = (e: KeyboardEvent) => {
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        void darUnPaso();
      }
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, [darUnPaso]);

  return (
    <div
      style={{ left, top }}
      className={`absolute z-50 -translate-x-1/2 ${flipped ? "-translate-y-full" : ""}`}
      onPointerDown={(e) => e.stopPropagation()}
    >
      <GlassPanel className="flex items-center gap-2 px-2 py-1.5">
        <span className="flex items-center gap-1.5 px-1 text-[12px] text-neutral-300">
          <ScrollText className="size-3.5 text-sky-400" />
          <span className="tabular-nums">
            {pasos > 0 ? `${pasos} ${t("fotos")}` : t("Captura larga")}
          </span>
        </span>

        {alto > 0 && (
          <span className="px-1 text-[11px] tabular-nums text-neutral-500">
            {alto} px
          </span>
        )}

        {trabajando && <Loader2 className="size-3.5 animate-spin text-neutral-400" />}

        <span className="mx-0.5 h-5 w-px bg-white/10" />

        <button
          type="button"
          onClick={onFinish}
          className="flex h-8 items-center gap-1.5 rounded-lg bg-sky-500 px-3 text-[13px] font-semibold text-white transition-colors hover:bg-sky-400"
        >
          <Check className="size-3.5" />
          {t("Terminar")}
        </button>

        <button
          type="button"
          onClick={onCancel}
          title={t("Cancelar")}
          className="flex size-8 items-center justify-center rounded-lg text-neutral-400 transition-colors hover:bg-white/10 hover:text-red-300"
        >
          <X className="size-4" />
        </button>
      </GlassPanel>

      {/*
        El recordatorio de como se barre. Se quita en cuanto llega la primera foto con
        algo nuevo: ahi ya se ha visto solo que la rueda cose, y una barra que repite lo
        que el usuario acaba de aprender estorba mas de lo que ayuda.
      */}
      {anadidas === 0 && pasos <= 1 && !error && (
        <div className="mt-1.5 flex justify-center">
          <span className="rounded-lg bg-neutral-900/90 px-2.5 py-1 text-[11px] text-neutral-300 shadow-lg">
            {t("Desplaza para coser más · Enter da un paso · Esc termina")}
          </span>
        </div>
      )}

      {anadidas > 0 && (
        <div className="mt-1.5 flex justify-center">
          <span className="rounded-lg bg-emerald-950/85 px-2 py-1 text-[11px] text-emerald-300 shadow-lg tabular-nums">
            +{anadidas} px
          </span>
        </div>
      )}

      {error && (
        <div className="mt-1.5 flex justify-center">
          <span className="max-w-md rounded-lg bg-red-950/90 px-2 py-1 text-[11px] text-red-200 shadow-lg">
            {t(error)}
          </span>
        </div>
      )}
    </div>
  );
}
