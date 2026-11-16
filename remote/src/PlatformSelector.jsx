import { tick } from './haptics.js';
import { Chevron } from './icons.jsx';

export function PlatformSelector({ platform, busy, disabled, onSelect }) {
  if (!platform?.options) return null;
  const known = platform.options.some((option) => option.id === platform.active);
  return <span className={`platform-selector ${busy ? 'is-switching' : ''}`}>
    <select aria-label="TV platform" value={known ? platform.active : ''} disabled={disabled || busy}
      onChange={(event) => { tick(); onSelect(event.target.value); }}>
      {!known && <option value="" disabled>{platform.label || 'Choose a platform'}</option>}
      {platform.options.map((option) => <option key={option.id} value={option.id} disabled={option.ready === false}>
        {option.name}{option.ready === false ? ' (finish setup)' : ''}
      </option>)}
    </select>
    <Chevron size={14} aria-hidden="true" />
    {busy && <span className="platform-switching" role="status">Switching…</span>}
  </span>;
}
