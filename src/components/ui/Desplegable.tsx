import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";

interface Props<T extends string | number> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  /** Cómo se llama el desplegable entero, para quien lo oye. */
  etiqueta?: string;
  /** Ancho del botón cerrado. Por defecto, el de su contenido. */
  ancho?: string;
  /**
   * Abre la lista hacia arriba.
   *
   * Lo usa la barra de abajo, que vive en el borde inferior de la ventana: una lista que
   * se abre hacia abajo ahí sale de la ventana y no se ve.
   */
  haciaArriba?: boolean;
}

/**
 * Selector de muchas opciones: se abre una lista y se elige una.
 *
 * Existe porque un `Segmented` deja de valer cuando las opciones dejan de caber. Con
 * trece idiomas, trece botones en fila pedían más de mil píxeles y la ventana mide 840:
 * el control se salía, empujaba la rejilla de dos columnas y la columna de la izquierda
 * acababa fuera de la ventana. Una lista no crece a lo ancho por tener más opciones.
 *
 * No es un `<select>` nativo porque el nativo lo pinta Windows y desentona con el resto
 * de la ventana; pero se comporta como uno: se cierra con Escape, se cierra al pulsar
 * fuera, y cada opción dice si está elegida con `aria-selected`.
 */
export function Desplegable<T extends string | number>({
  value,
  options,
  onChange,
  etiqueta,
  ancho,
  haciaArriba = false,
}: Props<T>) {
  const [abierto, setAbierto] = useState(false);
  const caja = useRef<HTMLDivElement>(null);

  const elegida = options.find((o) => o.value === value);

  // Cerrar al pulsar fuera. Se escucha en `pointerdown` y no en `click`: con `click`, el
  // que empieza dentro del panel y suelta fuera lo cerraría sin haber elegido nada.
  useEffect(() => {
    if (!abierto) return;
    const fuera = (e: PointerEvent) => {
      if (caja.current && !caja.current.contains(e.target as Node)) setAbierto(false);
    };
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setAbierto(false);
      }
    };
    document.addEventListener("pointerdown", fuera);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", fuera);
      document.removeEventListener("keydown", escape);
    };
  }, [abierto]);

  return (
    <div ref={caja} className={`relative ${ancho ?? "inline-block"}`}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={abierto}
        aria-label={etiqueta}
        onClick={() => setAbierto((v) => !v)}
        className="flex w-full items-center justify-between gap-2 rounded-lg bg-hueco px-3 py-1.5 text-xs font-medium text-titulo transition-colors hover:bg-pastilla"
      >
        <span className="truncate">{elegida?.label ?? String(value)}</span>
        <ChevronDown
          className={`size-3.5 shrink-0 text-apagado transition-transform ${abierto ? "rotate-180" : ""}`}
        />
      </button>

      {abierto && (
        <div
          role="listbox"
          aria-label={etiqueta}
          // `max-h` con scroll: trece idiomas caben de sobra, pero el día que haya veinte
          // la lista no puede salirse de la ventana por abajo.
          className={`absolute right-0 z-50 max-h-72 w-full min-w-44 overflow-y-auto rounded-lg border border-linea-fuerte bg-flotante p-1 shadow-2xl ${
            haciaArriba ? "bottom-full mb-1" : "top-full mt-1"
          }`}
        >
          {options.map((option) => {
            const activa = option.value === value;
            return (
              <button
                key={String(option.value)}
                type="button"
                role="option"
                aria-selected={activa}
                onClick={() => {
                  onChange(option.value);
                  setAbierto(false);
                }}
                className={`flex w-full items-center justify-between gap-2 rounded-md px-2.5 py-1.5 text-left text-xs transition-colors ${
                  activa
                    ? "bg-pastilla font-medium text-titulo"
                    : "text-apagado hover:bg-hueco hover:text-titulo"
                }`}
              >
                <span className="truncate">{option.label}</span>
                {activa && <Check className="size-3.5 shrink-0" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
