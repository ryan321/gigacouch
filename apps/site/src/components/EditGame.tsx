"use client";

import type { FormEvent } from "react";
import type { GameDefaults } from "./GameFields";
import { Field } from "./Field";
import { GameFields } from "./GameFields";
import { UploadProgress } from "./UploadForm";
import { useSubmit } from "./send";

type Editable = GameDefaults & { id: string; title: string; cover: string | null; screenshots: string[] };

export function EditGameForm({ game }: { game: Editable }) {
  const { errors, pending, submit } = useSubmit(`/api/games/${game.id}`);
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <GameFields defaults={game} errors={errors} />
      <div className="field">
        <span className="label">Cover image</span>
        {game.cover && (
          <div className="thumbs" style={{ marginBottom: 10 }}>
            <label>
              <img src={`/media/${game.cover}`} alt="The current cover" />
              <span className="check"><input type="checkbox" name="removeCover" /> Remove</span>
            </label>
          </div>
        )}
        <input type="file" name="cover" accept="image/png,image/jpeg,image/gif,image/webp" />
        {errors.cover && <span className="field-error">{errors.cover}</span>}
      </div>
      <div className="field">
        <span className="label">Screenshots</span>
        {game.screenshots.length > 0 && (
          <div className="thumbs" style={{ marginBottom: 10 }}>
            {game.screenshots.map((name, index) => (
              <label key={name}>
                <img src={`/media/${name}`} alt={`Screenshot ${index + 1}`} />
                <span className="check"><input type="checkbox" name="removeScreenshot" value={name} /> Remove</span>
              </label>
            ))}
          </div>
        )}
        <input type="file" name="screenshots" accept="image/png,image/jpeg,image/gif,image/webp" multiple />
        {errors.screenshots && <span className="field-error">{errors.screenshots}</span>}
      </div>
      <div className="form-actions">
        <button className="button" type="submit" disabled={pending}>{pending ? "Saving…" : "Save changes"}</button>
      </div>
    </form>
  );
}

export function NewBuildForm({ gameId }: { gameId: string }) {
  const { errors, pending, progress, submit } = useSubmit(`/api/games/${gameId}/builds`);
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <Field label="New build" hint="Players get it as soon as it's checked. The old build is kept." error={errors.build}>
        <input type="file" name="build" accept=".zip,application/zip" required aria-invalid={Boolean(errors.build)} />
      </Field>
      <div className="form-actions">
        <button className="button secondary" type="submit" disabled={pending}>Upload new build</button>
        <UploadProgress pending={pending} progress={progress} verb="Uploading" />
      </div>
    </form>
  );
}

export function DeleteGameButton({ gameId, title }: { gameId: string; title: string }) {
  const { errors, pending, submit } = useSubmit(`/api/games/${gameId}`, "DELETE");
  return (
    <div className="form-actions">
      <button
        type="button"
        className="button danger"
        disabled={pending}
        onClick={() => {
          if (window.confirm(`Delete ${title}? Its page, builds, and images are removed for good.`)) submit(null);
        }}
      >
        Delete game
      </button>
      {errors.form && <span className="field-error">{errors.form}</span>}
    </div>
  );
}
