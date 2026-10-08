"use client";

import type { FormEvent } from "react";
import { Field } from "./Field";
import { GameFields } from "./GameFields";
import { useSubmit } from "./send";

export function UploadProgress({ pending, progress, verb }: { pending: boolean; progress: number | null; verb: string }) {
  if (!pending) return null;
  if (progress !== null && progress < 1) {
    return (
      <>
        <div className="progress" aria-hidden="true"><i style={{ width: `${Math.round(progress * 100)}%` }} /></div>
        <span className="muted" role="status">{verb} {Math.round(progress * 100)}%</span>
      </>
    );
  }
  return <span className="muted" role="status">Checking your game…</span>;
}

export function UploadForm() {
  const { errors, pending, progress, submit } = useSubmit("/api/games");
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <Field label="Game build" hint="A .zip of your web export, up to 300 MB." error={errors.build}>
        <input type="file" name="build" accept=".zip,application/zip" required aria-invalid={Boolean(errors.build)} />
      </Field>
      <GameFields errors={errors} />
      <Field label="Cover image" hint="Shown on game tiles. 16:9 looks best, such as 1280 by 720. Without one, the title is shown instead." error={errors.cover}>
        <input type="file" name="cover" accept="image/png,image/jpeg,image/gif,image/webp" />
      </Field>
      <Field label="Screenshots" hint="Up to 6 images for the game's page." error={errors.screenshots}>
        <input type="file" name="screenshots" accept="image/png,image/jpeg,image/gif,image/webp" multiple />
      </Field>
      <div className="form-actions">
        <button className="button" type="submit" disabled={pending}>Upload game</button>
        <UploadProgress pending={pending} progress={progress} verb="Uploading" />
      </div>
    </form>
  );
}
