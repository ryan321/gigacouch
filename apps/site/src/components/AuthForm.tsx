"use client";

import Link from "next/link";
import type { FormEvent } from "react";
import { Field } from "./Field";
import { useSubmit } from "./send";

export function SignUpForm({ next }: { next: string }) {
  const { errors, pending, submit } = useSubmit("/api/auth/signup");
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form narrow" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <input type="hidden" name="next" value={next} />
      <Field label="Email" error={errors.email}>
        <input type="email" name="email" autoComplete="email" required aria-invalid={Boolean(errors.email)} />
      </Field>
      <Field label="Handle" hint="Your address on Giga Couch, like gigacouch.com/u/yourname." error={errors.handle}>
        <input type="text" name="handle" autoComplete="username" required pattern="[a-z0-9_]{3,20}" aria-invalid={Boolean(errors.handle)} />
      </Field>
      <Field label="Display name" hint="The name people see on your games. You can change it later." error={errors.displayName}>
        <input type="text" name="displayName" autoComplete="nickname" maxLength={40} aria-invalid={Boolean(errors.displayName)} />
      </Field>
      <Field label="Password" hint="At least 10 characters." error={errors.password}>
        <input type="password" name="password" autoComplete="new-password" required minLength={10} aria-invalid={Boolean(errors.password)} />
      </Field>
      <div>
        <label className="check">
          <input type="checkbox" name="oldEnough" /> I&apos;m 13 or older
        </label>
        {errors.oldEnough && <span className="field-error">{errors.oldEnough}</span>}
      </div>
      <div className="form-actions">
        <button className="button" type="submit" disabled={pending}>{pending ? "Creating your account…" : "Create account"}</button>
      </div>
      <p className="switch">
        Already have an account? <Link href={`/signin?next=${encodeURIComponent(next)}`}>Sign in</Link>
      </p>
    </form>
  );
}

export function SignInForm({ next }: { next: string }) {
  const { errors, pending, submit } = useSubmit("/api/auth/signin");
  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    submit(new FormData(event.currentTarget));
  };
  return (
    <form className="form narrow" onSubmit={onSubmit} noValidate>
      {errors.form && <p className="form-error" role="alert">{errors.form}</p>}
      <input type="hidden" name="next" value={next} />
      <Field label="Email or handle">
        <input type="text" name="login" autoComplete="username" required />
      </Field>
      <Field label="Password">
        <input type="password" name="password" autoComplete="current-password" required />
      </Field>
      <div className="form-actions">
        <button className="button" type="submit" disabled={pending}>{pending ? "Signing in…" : "Sign in"}</button>
      </div>
      <p className="switch">
        New here? <Link href={`/signup?next=${encodeURIComponent(next)}`}>Create an account</Link>
      </p>
    </form>
  );
}
