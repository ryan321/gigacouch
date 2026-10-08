import { mediaUrl, tintFor } from "@/lib/format";

export function Avatar({ user, size = 32 }: { user: { id: string; displayName: string; avatar: string | null }; size?: number }) {
  const url = mediaUrl(user.avatar);
  if (url) return <img className="avatar" src={url} alt="" width={size} height={size} style={{ width: size, height: size }} />;
  return (
    <span className="avatar" aria-hidden="true" style={{ width: size, height: size, background: tintFor(user.id), fontSize: size * 0.45 }}>
      {user.displayName.slice(0, 1).toUpperCase()}
    </span>
  );
}
