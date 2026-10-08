import { Field } from "./Field";

export type GameDefaults = {
  title: string;
  description: string;
  tags: string[];
  playersMin: number;
  playersMax: number;
  gamepad: boolean;
  keyboard: boolean;
  mouse: boolean;
  visibility: "public" | "unlisted" | "draft";
};

const BLANK: GameDefaults = {
  title: "",
  description: "",
  tags: [],
  playersMin: 1,
  playersMax: 1,
  gamepad: true,
  keyboard: true,
  mouse: false,
  visibility: "public",
};

const PLAYER_COUNTS = [1, 2, 3, 4];

/** The details every game has, shared by the upload and edit forms. */
export function GameFields({ defaults = BLANK, errors }: { defaults?: GameDefaults; errors: Record<string, string> }) {
  return (
    <>
      <Field label="Title" error={errors.title}>
        <input type="text" name="title" defaultValue={defaults.title} maxLength={60} required aria-invalid={Boolean(errors.title)} />
      </Field>
      <Field label="Description" hint="What the game is, and how to play it." error={errors.description}>
        <textarea name="description" defaultValue={defaults.description} maxLength={4000} aria-invalid={Boolean(errors.description)} />
      </Field>
      <Field label="Tags" hint="Up to 5, separated by commas, like racing, party, puzzle." error={errors.tags}>
        <input type="text" name="tags" defaultValue={defaults.tags.join(", ")} aria-invalid={Boolean(errors.tags)} />
      </Field>
      <fieldset>
        <legend>Players</legend>
        <div className="row">
          <Field label="Fewest">
            <select name="playersMin" defaultValue={defaults.playersMin}>
              {PLAYER_COUNTS.map((count) => <option key={count} value={count}>{count}</option>)}
            </select>
          </Field>
          <Field label="Most">
            <select name="playersMax" defaultValue={defaults.playersMax}>
              {PLAYER_COUNTS.map((count) => <option key={count} value={count}>{count}</option>)}
            </select>
          </Field>
        </div>
        {errors.players && <span className="field-error">{errors.players}</span>}
      </fieldset>
      <fieldset>
        <legend>How people play it</legend>
        <div className="checks">
          <label className="check"><input type="checkbox" name="gamepad" defaultChecked={defaults.gamepad} /> Controller</label>
          <label className="check"><input type="checkbox" name="keyboard" defaultChecked={defaults.keyboard} /> Keyboard</label>
          <label className="check"><input type="checkbox" name="mouse" defaultChecked={defaults.mouse} /> Mouse</label>
        </div>
        {errors.controls && <span className="field-error">{errors.controls}</span>}
      </fieldset>
      <Field label="Who can see it" error={errors.visibility}>
        <select name="visibility" defaultValue={defaults.visibility}>
          <option value="public">Everyone: it shows up in browsing and search</option>
          <option value="unlisted">Anyone with the link</option>
          <option value="draft">Only me, as a draft</option>
        </select>
      </Field>
    </>
  );
}
