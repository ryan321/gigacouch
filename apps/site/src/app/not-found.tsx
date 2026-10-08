import Link from "next/link";

export default function NotFound() {
  return (
    <main className="page auth">
      <h1>That page isn&apos;t here</h1>
      <p className="lede" style={{ margin: "12px 0 28px" }}>The game or person may have moved, or it was never public.</p>
      <div className="form-actions">
        <Link className="button" href="/games">Browse games</Link>
        <Link className="button secondary" href="/">Go to the home page</Link>
      </div>
    </main>
  );
}
