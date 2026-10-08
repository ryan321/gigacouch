"use client";

import type { FormEvent } from "react";
import type { User } from "@/lib/auth";
import { Avatar } from "./Avatar";
import { Field } from "./Field";
import { useSubmit } from "./send";

export function ProfileForm({ user }: { user: User }) {
  const { errors, pending, submit } = useSubmit("/api/profile");
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <Field label="Display name" error={errors.displayName}>
        <input type="text" name="displayName" defaultValue={user.displayName} maxLength={40} required aria-invalid={Boolean(errors.displayName)} />
      </Field>
      <Field label="Bio" hint="A line or two about you and the games you make." error={errors.bio}>
        <textarea name="bio" defaultValue={user.bio} maxLength={300} rows={4} aria-invalid={Boolean(errors.bio)} />
      </Field>
      <div className="field">
        <span className="label">Picture</span>
        <div className="row" style={{ alignItems: "center" }}>
          <Avatar user={user} size={64} />
          <input type="file" name="avatar" accept="image/png,image/jpeg,image/gif,image/webp" />
        </div>
        {user.avatar && (
          <label className="check" style={{ marginTop: 10 }}>
            <input type="checkbox" name="removeAvatar" /> Remove my picture
          </label>
        )}
        {errors.avatar && <span className="field-error">{errors.avatar}</span>}
      </div>
      <p className="muted">Your handle is @{user.handle}. Your email is {user.email}, and only you can see it.</p>
      <div className="form-actions">
        <button className="button" type="submit" disabled={pending}>{pending ? "Saving…" : "Save profile"}</button>
      </div>
    </form>
  );
}
