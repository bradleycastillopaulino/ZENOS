import { useEffect, useState } from "react";
import { api, type ConfirmInfo } from "../api";
import { useAppConfig } from "../theme";
import "./settings.css";

/** Ventana que pide permiso antes de una acción delicada. Cerrarla = no autorizar. */
export function Confirm() {
  const [info, setInfo] = useState<ConfirmInfo | null>(null);
  useAppConfig();
  useEffect(() => {
    document.documentElement.classList.add("settings");
    void api.getConfirm().then(setInfo);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && void api.answerConfirm(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  if (!info) return null;
  return (
    <div className="confirm">
      <div className={info.danger ? "badge danger" : "badge"}>{info.danger ? "Acción delicada" : "Confirmar acción"}</div>
      <h2>ZENOS quiere hacer esto:</h2>
      <p className="what">{info.text}</p>
      <div className="nav">
        <button className="ghost" autoFocus onClick={() => void api.answerConfirm(false)}>
          Cancelar
        </button>
        <button className={info.danger ? "danger-btn" : ""} onClick={() => void api.answerConfirm(true)}>
          Permitir
        </button>
      </div>
    </div>
  );
}
