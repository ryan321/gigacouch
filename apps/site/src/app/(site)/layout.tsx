import Link from "next/link";
import type { ReactNode } from "react";
import { SiteHeader } from "@/components/SiteHeader";
import { keepPagesOnSite } from "@/lib/site-host";

export const dynamic = "force-dynamic";

export default async function SiteLayout({ children }: { children: ReactNode }) {
  await keepPagesOnSite();
  return (
    <>
      <SiteHeader />
      <main>{children}</main>
      <footer className="site-footer">
        <div className="page">
          <p>Play web games with a controller, keyboard, mouse, or a game's touch controls. Mobile support depends on the game.</p>
          <p>
            <Link href="/games">Browse games</Link>
          </p>
        </div>
      </footer>
    </>
  );
}
