import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { SignInForm } from "@/components/AuthForm";
import { currentUser } from "@/lib/auth";
import { safeNext } from "@/lib/next-path";
import { one, type SearchParams } from "@/lib/params";

export const metadata: Metadata = { title: "Sign in" };

export default async function SignInPage({ searchParams }: { searchParams: SearchParams }) {
  const next = safeNext(one((await searchParams).next));
  if (await currentUser()) redirect(next);
  return (
    <div className="page auth">
      <h1>Sign in</h1>
      <p className="lede" style={{ marginBottom: 28 }}>Welcome back.</p>
      <SignInForm next={next} />
    </div>
  );
}
