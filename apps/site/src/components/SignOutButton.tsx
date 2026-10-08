"use client";

import { useSubmit } from "./send";

export function SignOutButton() {
  const { pending, submit } = useSubmit("/api/auth/signout");
  return (
    <button type="button" className="link-button" disabled={pending} onClick={() => submit(null)}>
      Sign out
    </button>
  );
}
