import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { UploadForm } from "@/components/UploadForm";
import { currentUser } from "@/lib/auth";

export const metadata: Metadata = { title: "Upload a game" };

export default async function UploadPage() {
  const user = await currentUser();
  if (!user) redirect("/signin?next=/upload");
  return (
    <div className="page">
      <div className="page-head">
        <h1>Upload a game</h1>
        <p className="lede">Any game that runs in a web page can go here.</p>
      </div>
      <div className="panel-box form" style={{ marginBottom: 32 }}>
        <h2>Before you upload</h2>
        <ul className="requirements">
          <li>Zip your engine&apos;s web export with index.html at the top level of the zip.</li>
          <li>Godot: in the Web export options, turn off Thread Support.</li>
          <li>Unity: Brotli and Gzip builds both work.</li>
          <li>Read controllers with the standard gamepad API. Chrome shows up to four pads.</li>
          <li>Save progress in the browser, with localStorage or IndexedDB.</li>
        </ul>
      </div>
      <UploadForm />
    </div>
  );
}
