// The square image of a mod or profile, or the first letter of the name when there is none. With `pick`, a click
// chooses a new profile image and a small button removes it. With `zoom`, a click on an image opens it large.
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { run } from "./api";
import { CloseIcon } from "./Icons";
import { t } from "./i18n";
import type { ImageRef } from "./types";

const TYPES = ["png", "jpg", "jpeg", "webp"];

export const imageSrc = (img: ImageRef) => `${convertFileSrc(img.path)}?v=${img.v}`;

type Pick = { key: string; custom: boolean };

export function Thumb({ image, name, size = "md", pick, zoom }: { image: ImageRef | null; name: string; size?: "sm" | "md" | "lg"; pick?: Pick; zoom?: boolean }) {
  const face = image ? <img src={imageSrc(image)} alt="" draggable={false} /> : <span aria-hidden="true">{[...name.trim()][0]?.toUpperCase() ?? "?"}</span>;
  if (zoom && image) return <ZoomThumb image={image} size={size}>{face}</ZoomThumb>;
  if (!pick) return <span className={`thumb ${size}`}>{face}</span>;
  const choose = async () => {
    const path = await open({ multiple: false, directory: false, title: t("app.image.choose"), filters: [{ name: t("app.image.files"), extensions: TYPES }] });
    if (typeof path === "string") run("set_image", { key: pick.key, path });
  };
  return (
    <span className="thumb-wrap">
      <button className={`thumb ${size}`} onClick={choose} aria-label={t("app.image.choose")} data-tip-title={t("app.image.choose")} data-tip={t("app.image.chooseHint")}>
        {face}
      </button>
      {pick.custom && (
        <button className="thumb-x" onClick={() => run("set_image", { key: pick.key, path: null })}
          aria-label={t("app.image.remove")} data-tip={t("app.image.remove")}><CloseIcon /></button>
      )}
    </span>
  );
}

function ZoomThumb({ image, size, children }: { image: ImageRef; size: string; children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button className={`thumb zoom ${size}`} onClick={() => setOpen(true)} aria-haspopup="dialog" aria-label={t("app.image.view")} data-tip={t("app.image.view")}>
        {children}
      </button>
      {open && <ImageViewer image={image} onClose={() => setOpen(false)} />}
    </>
  );
}

/** One image, as large as the window allows. A click anywhere, or Esc, closes it. */
export function ImageViewer({ image, onClose }: { image: ImageRef; onClose: () => void }) {
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    close.current?.focus();
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose]);
  return createPortal(
    <div className="modal-back image-view" role="dialog" aria-modal="true" aria-label={t("app.image.view")} onClick={onClose}>
      <img src={imageSrc(image)} alt="" draggable={false} />
      <button className="bare image-view-x" ref={close} aria-label={t("app.close")} data-tip={t("app.close")}><CloseIcon /></button>
    </div>,
    document.body,
  );
}
