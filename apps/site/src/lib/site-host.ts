import { headers } from "next/headers";
import { redirect } from "next/navigation";
import { GAMES_ORIGIN, SITE_ORIGIN, hostOf } from "./config";

/** Pages belong on the site's origin. On the games origin, send people back. */
export async function keepPagesOnSite(): Promise<void> {
  if (hostOf(GAMES_ORIGIN) === hostOf(SITE_ORIGIN)) return;
  if ((await headers()).get("host") === hostOf(GAMES_ORIGIN)) redirect(SITE_ORIGIN);
}
