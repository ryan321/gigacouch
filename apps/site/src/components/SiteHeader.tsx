import Link from "next/link";
import { currentUser } from "@/lib/auth";
import { Avatar } from "./Avatar";
import { SignOutButton } from "./SignOutButton";

export async function SiteHeader() {
  const user = await currentUser();
  return (
    <header className="site-header">
      <div className="page">
        <Link className="brand" href="/">
          <img src="/brand/mark.png" alt="" width={34} height={34} />
          <span className="wordmark">Giga Couch</span>
        </Link>
        <nav className="nav" aria-label="Main">
          <Link href="/games">Games</Link>
          <Link href="/upload" className="wide">Upload a game</Link>
        </nav>
        <div className="account">
          {user ? (
            <>
              <Link className="who" href={`/u/${user.handle}`}>
                <Avatar user={user} size={30} />
                <span className="name">{user.displayName}</span>
              </Link>
              <Link className="link-button" href="/settings">Settings</Link>
              <SignOutButton />
            </>
          ) : (
            <>
              <Link className="link-button" href="/signin">Sign in</Link>
              <Link className="button small" href="/signup">Sign up</Link>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
