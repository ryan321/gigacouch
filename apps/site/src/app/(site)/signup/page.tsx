import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { SignUpForm } from "@/components/AuthForm";
import { currentUser } from "@/lib/auth";
import { safeNext } from "@/lib/next-path";
import { one, type SearchParams } from "@/lib/params";

export const metadata: Metadata = { title: "Sign up" };

export default async function SignUpPage({ searchParams }: { searchParams: SearchParams }) {
  const next = safeNext(one((await searchParams).next), "");
  const user = await currentUser();
  if (user) redirect(next || `/u/${user.handle}`);
  return (
    <div className="page auth">
      <h1>Make your account</h1>
      <p className="lede" style={{ marginBottom: 28 }}>Upload games, keep a profile, and see what you&apos;ve made.</p>
      <SignUpForm next={next} />
    </div>
  );
}
