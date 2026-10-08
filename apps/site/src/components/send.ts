"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

export type Result = { ok: boolean; errors?: Record<string, string>; redirect?: string };

/** Send a form to one of the site's API routes, reporting upload progress. */
export function send(url: string, body: FormData | null, method = "POST", onProgress?: (fraction: number) => void): Promise<Result> {
  return new Promise((resolve) => {
    const request = new XMLHttpRequest();
    request.open(method, url);
    request.responseType = "text";
    if (onProgress) request.upload.onprogress = (event) => event.lengthComputable && onProgress(event.loaded / event.total);
    request.onload = () => {
      try {
        resolve(JSON.parse(request.responseText) as Result);
      } catch {
        resolve({ ok: false, errors: { form: `The server couldn't handle that (error ${request.status}). Try again.` } });
      }
    };
    request.onerror = () => resolve({ ok: false, errors: { form: "Couldn't reach Giga Couch. Check your connection and try again." } });
    request.send(body);
  });
}

/** Form state for a form that posts to an API route and then moves on. */
export function useSubmit(url: string, method = "POST") {
  const router = useRouter();
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);

  async function submit(body: FormData | null) {
    setPending(true);
    setErrors({});
    setProgress(null);
    const result = await send(url, body, method, setProgress);
    if (result.ok) {
      router.push(result.redirect ?? "/");
      router.refresh();
      return;
    }
    setErrors(result.errors ?? { form: "That didn't work. Try again." });
    setPending(false);
    setProgress(null);
  }

  return { errors, pending, progress, submit };
}
