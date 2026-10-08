import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { ProfileForm } from "@/components/ProfileForm";
import { currentUser } from "@/lib/auth";

export const metadata: Metadata = { title: "Settings" };

export default async function SettingsPage() {
  const user = await currentUser();
  if (!user) redirect("/signin?next=/settings");
  return (
    <div className="page">
      <div className="page-head">
        <h1>Your profile</h1>
        <p className="lede">This is what people see on your page and next to your games.</p>
      </div>
      <ProfileForm user={user} />
    </div>
  );
}
