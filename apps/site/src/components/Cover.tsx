import type { CSSProperties } from "react";
import { mediaUrl, tintFor } from "@/lib/format";

/** A game's cover, or its title set in type when it has no cover. */
export function Cover({ game, large = false }: { game: { id: string; title: string; cover: string | null }; large?: boolean }) {
  const url = mediaUrl(game.cover);
  return (
    <div className={large ? "cover large" : "cover"}>
      {url ? (
        <img src={url} alt="" />
      ) : (
        <div className="cover-fallback" style={{ "--tint": tintFor(game.id) } as CSSProperties}>
          <span>{game.title}</span>
        </div>
      )}
    </div>
  );
}
