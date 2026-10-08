import type { Metadata } from "next";
import Link from "next/link";
import { notFound, redirect } from "next/navigation";
import { DeleteGameButton, EditGameForm, NewBuildForm } from "@/components/EditGame";
import { currentUser } from "@/lib/auth";
import { gameBySlug } from "@/lib/games";

export const metadata: Metadata = { title: "Edit game" };

export default async function EditGamePage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const user = await currentUser();
  if (!user) redirect(`/signin?next=${encodeURIComponent(`/games/${slug}/edit`)}`);
  const game = gameBySlug(slug);
  if (!game || game.owner.id !== user.id) notFound();
  return (
    <div className="page">
      <div className="page-head">
        <Link className="muted" href={`/games/${game.slug}`}>Back to {game.title}</Link>
        <h1 style={{ marginTop: 10 }}>Edit {game.title}</h1>
      </div>
      <EditGameForm game={game} />
      <section className="section" style={{ marginTop: 40 }}>
        <h2 style={{ marginBottom: 14 }}>Upload a new build</h2>
        <NewBuildForm gameId={game.id} />
      </section>
      <section className="section">
        <h2 style={{ marginBottom: 8 }}>Delete this game</h2>
        <p className="muted" style={{ marginBottom: 14 }}>This removes the game&apos;s page, every build, and its images. It can&apos;t be undone.</p>
        <DeleteGameButton gameId={game.id} title={game.title} />
      </section>
    </div>
  );
}
