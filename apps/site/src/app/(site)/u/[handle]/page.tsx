import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Avatar } from "@/components/Avatar";
import { GameTiles } from "@/components/GameTile";
import { currentUser, userByHandle } from "@/lib/auth";
import { gamesByOwner } from "@/lib/games";

type Props = { params: Promise<{ handle: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const person = userByHandle((await params).handle);
  return person ? { title: `${person.displayName} (@${person.handle})` } : {};
}

export default async function ProfilePage({ params }: Props) {
  const person = userByHandle((await params).handle);
  if (!person) notFound();
  const viewer = await currentUser();
  const mine = viewer?.id === person.id;
  const games = gamesByOwner(person.id, mine);
  return (
    <div className="page">
      <div className="profile-head">
        <Avatar user={person} size={96} />
        <div className="who">
          <h1>{person.displayName}</h1>
          <p className="muted">@{person.handle}</p>
          {person.bio && <p className="bio">{person.bio}</p>}
        </div>
        {mine && (
          <div className="actions">
            <Link className="button secondary" href="/settings">Edit profile</Link>
            <Link className="button" href="/upload">Upload a game</Link>
          </div>
        )}
      </div>
      <section className="section">
        <div className="section-head">
          <h2>{mine ? "Your games" : "Games"}</h2>
        </div>
        {games.length > 0 ? (
          <GameTiles games={games} showOwner={false} />
        ) : mine ? (
          <div className="empty">
            <p>You haven&apos;t uploaded a game yet. Zip a web export and put it up in a couple of minutes.</p>
            <Link className="button" href="/upload">Upload a game</Link>
          </div>
        ) : (
          <div className="empty">
            <p>{person.displayName} hasn&apos;t published a game yet.</p>
          </div>
        )}
      </section>
    </div>
  );
}
