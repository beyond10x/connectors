// In-page probes of the layout contract. Each function is passed to page.evaluate, so it is
// self-contained: no imports, no closures over module scope.

export type Overflow = {kind: 'page-scroll-x' | 'runs-past' | 'clipped' | 'past-viewport'; path: string; detail: string};

// Gate 1 in docs-site form: no horizontal page scroll, and no element running past its box. A box
// that scrolls on its own (overflow auto or scroll) holds its content and passes; text cut with an
// ellipsis passes only when its full value is available as a title or an accessible name.
export function overflowProbe(): Overflow[] {
  const describe = (el: Element) => {
    const parts: string[] = [];
    for (let e: Element | null = el; e && e !== document.body && parts.length < 4; e = e.parentElement) {
      const cls = [...e.classList].filter((c) => !/_[A-Za-z0-9]{4}$/.test(c)).slice(0, 2).join('.');
      parts.unshift(e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (cls ? '.' + cls : ''));
    }
    return parts.join(' > ');
  };
  const out: Overflow[] = [];
  const width = window.innerWidth;
  const root = document.documentElement;
  if (root.scrollWidth > width + 1) out.push({kind: 'page-scroll-x', path: 'html', detail: `scrollWidth ${root.scrollWidth} > ${width}`});
  const hidden = (el: Element) => {
    for (let e: Element | null = el; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.display === 'none' || cs.visibility === 'hidden' || cs.contentVisibility === 'hidden') return true;
      if (e.getAttribute('aria-hidden') === 'true' && e !== el) return true;
    }
    return false;
  };
  const scrollsX = (el: Element | null): boolean => {
    for (let e = el; e && e !== document.documentElement; e = e.parentElement) {
      const ox = getComputedStyle(e).overflowX;
      if (ox === 'auto' || ox === 'scroll') return true;
    }
    return false;
  };
  // A child that sticks out on purpose: a negative-margin row (the grid's gutter bleed) or a
  // transformed decoration, which does not move the layout.
  const bleeds = (c: Element) => {
    const cs = getComputedStyle(c);
    return parseFloat(cs.marginLeft) < 0 || parseFloat(cs.marginRight) < 0 || cs.transform !== 'none' || cs.position === 'fixed';
  };
  const range = document.createRange();
  for (const el of document.body.querySelectorAll('*')) {
    if (!(el instanceof HTMLElement) || el.clientWidth <= 1) continue;
    const rect = el.getBoundingClientRect();
    const pastViewport = rect.width > 0 && rect.right > width + 1 && rect.left < width;
    const over = el.scrollWidth - el.clientWidth;
    if (!pastViewport && over <= 1) continue;
    const cs = getComputedStyle(el);
    if (cs.position === 'absolute' && (cs.clip !== 'auto' || cs.clipPath !== 'none')) continue;
    if (hidden(el)) continue;
    if (pastViewport && !scrollsX(el.parentElement) && !(el.parentElement && el.parentElement.getBoundingClientRect().right > width + 1)) {
      out.push({kind: 'past-viewport', path: describe(el), detail: `right ${Math.round(rect.right)} > ${width}`});
      continue;
    }
    const ox = cs.overflowX;
    if (over <= 1 || ox === 'auto' || ox === 'scroll') continue;
    if (ox === 'hidden' || ox === 'clip') {
      const named = el.getAttribute('title') || el.getAttribute('aria-label') || el.closest('[title]');
      if (cs.textOverflow === 'ellipsis' && named) continue;
      // Clipping a box that holds no text of its own (a decoration, an image frame) loses nothing.
      if (!(el.innerText || '').trim()) continue;
      out.push({kind: 'clipped', path: describe(el), detail: `scrollWidth ${el.scrollWidth} > clientWidth ${el.clientWidth}, overflow ${ox}${cs.textOverflow === 'ellipsis' ? ', ellipsis without its full value' : ''}`});
      continue;
    }
    // Visible overflow: name the direct child or the text of this box that sticks out of it. A box
    // whose overflow comes from deeper down is reported at that deeper box instead.
    const left = rect.left + el.clientLeft - 1;
    const right = rect.left + el.clientLeft + el.clientWidth + 1;
    const sticks = (r: DOMRect) => r.width > 0 && (r.right > right || r.left < left);
    const offenders: string[] = [];
    for (const c of el.children) if (sticks(c.getBoundingClientRect()) && !bleeds(c) && !hidden(c)) offenders.push(describe(c).split(' > ').pop()!);
    for (const n of el.childNodes) {
      if (n.nodeType !== Node.TEXT_NODE || !(n.textContent ?? '').trim()) continue;
      range.selectNodeContents(n);
      if (sticks(range.getBoundingClientRect())) offenders.push(`text "${(n.textContent ?? '').trim().slice(0, 40)}"`);
    }
    if (offenders.length) out.push({kind: 'runs-past', path: describe(el), detail: `${offenders.slice(0, 3).join(', ')} past the box (scrollWidth ${el.scrollWidth} > clientWidth ${el.clientWidth})`});
  }
  return out;
}

export type Visibility = {present: boolean; inViewport: boolean; clippedBy: string | null; coveredAt: string[]; rect: [number, number, number, number] | null};

// Gate 2: the element is 100 % inside the viewport, not clipped by an ancestor and not covered at
// its centre or its four corners.
export function visibilityProbe(selector: string): Visibility {
  const el = [...document.querySelectorAll(selector)].find((e) => {
    const r = e.getBoundingClientRect();
    const cs = getComputedStyle(e);
    return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none';
  });
  if (!el) return {present: false, inViewport: false, clippedBy: null, coveredAt: [], rect: null};
  const r = el.getBoundingClientRect();
  const inViewport = r.left >= -0.5 && r.top >= -0.5 && r.right <= window.innerWidth + 0.5 && r.bottom <= window.innerHeight + 0.5;
  let clippedBy: string | null = null;
  for (let e = el.parentElement; e && e !== document.body; e = e.parentElement) {
    const cs = getComputedStyle(e);
    if (cs.overflowX === 'visible' && cs.overflowY === 'visible') continue;
    const a = e.getBoundingClientRect();
    if (r.left < a.left - 0.5 || r.top < a.top - 0.5 || r.right > a.right + 0.5 || r.bottom > a.bottom + 0.5) {
      clippedBy = e.tagName.toLowerCase() + (e.className ? '.' + String(e.className).split(' ')[0] : '');
      break;
    }
  }
  // Corners at a quarter of the box: a round button does not own the corners of its box.
  const ix = r.width / 4;
  const iy = r.height / 4;
  const points: [string, number, number][] = [
    ['centre', (r.left + r.right) / 2, (r.top + r.bottom) / 2],
    ['top-left', r.left + ix, r.top + iy],
    ['top-right', r.right - ix, r.top + iy],
    ['bottom-left', r.left + ix, r.bottom - iy],
    ['bottom-right', r.right - ix, r.bottom - iy],
  ];
  const coveredAt = points
    .filter(([, x, y]) => x >= 0 && y >= 0 && x < window.innerWidth && y < window.innerHeight)
    .filter(([, x, y]) => {
      const hit = document.elementFromPoint(x, y);
      return !hit || !(el === hit || el.contains(hit));
    })
    .map(([name]) => name);
  return {present: true, inViewport, clippedBy, coveredAt, rect: [r.left, r.top, r.right, r.bottom].map(Math.round) as [number, number, number, number]};
}

export type Control = {name: string; role: string; href: string | null; x: number; y: number; region: 'main' | 'navbar' | 'sidebar' | 'toc' | 'footer' | 'other'; inView: boolean; expanded: boolean};

// What the screen-only user can see: links and buttons with a name, fully inside the viewport and
// not covered at their centre. The name is the accessible name in its common forms: aria-label,
// aria-labelledby, else the rendered text, else the title.
export function controlsProbe(): Control[] {
  const name = (el: HTMLElement) => {
    const label = el.getAttribute('aria-label');
    if (label) return label.trim();
    const by = el.getAttribute('aria-labelledby');
    if (by) return by.split(/\s+/).map((id) => document.getElementById(id)?.innerText ?? '').join(' ').trim();
    const text = (el.innerText || '').replace(/\s+/g, ' ').trim();
    return text || (el.getAttribute('title') ?? '').trim();
  };
  const region = (el: Element): Control['region'] => {
    if (el.closest('nav.navbar, .navbar-sidebar')) return 'navbar';
    if (el.closest('aside.theme-doc-sidebar-container')) return 'sidebar';
    if (el.closest('.table-of-contents, .theme-doc-toc-desktop, .theme-doc-toc-mobile')) return 'toc';
    if (el.closest('footer')) return 'footer';
    if (el.closest('main')) return 'main';
    return 'other';
  };
  const out: Control[] = [];
  for (const el of document.querySelectorAll<HTMLElement>('a[href], button, [role="button"], [role="link"], summary')) {
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    const inView = r.left >= 0 && r.top >= 0 && r.right <= window.innerWidth && r.bottom <= window.innerHeight;
    const x = (r.left + r.right) / 2;
    const y = (r.top + r.bottom) / 2;
    if (!inView) continue;
    const hit = document.elementFromPoint(x, y);
    if (!hit || !(hit === el || el.contains(hit))) continue;
    const n = name(el);
    if (!n) continue;
    const role = el.tagName === 'A' || el.getAttribute('role') === 'link' ? 'link' : 'button';
    out.push({name: n, role, href: el instanceof HTMLAnchorElement ? el.href : null, x, y, region: region(el), inView, expanded: el.getAttribute('aria-expanded') === 'true'});
  }
  return out;
}

export type Goal = {found: boolean; visible: boolean; covered: boolean; top: number | null};

// Is the task's target on screen: the innermost element of the content whose text matches, fully
// inside the viewport and not covered (the sticky navbar is the usual cover).
export function goalProbe(arg: {source: string; flags: string}): Goal {
  const re = new RegExp(arg.source, arg.flags);
  const scope = document.querySelector('main') ?? document.body;
  let best: HTMLElement | null = null;
  for (const el of scope.querySelectorAll<HTMLElement>('*')) {
    const text = (el.textContent ?? '').replace(/\s+/g, ' ').trim();
    if (!re.test(text)) continue;
    if ([...el.children].some((c) => re.test((c.textContent ?? '').replace(/\s+/g, ' ').trim()))) continue;
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    best = el;
    if (r.top >= 0 && r.bottom <= window.innerHeight) break;
  }
  if (!best) return {found: false, visible: false, covered: false, top: null};
  const r = best.getBoundingClientRect();
  // On screen when its first line is: a reader sees a paragraph once it starts in view.
  const visible = r.top >= -0.5 && r.top + Math.min(r.height, 40) <= window.innerHeight + 0.5 && r.left >= -0.5 && r.left < window.innerWidth;
  const x = Math.min(window.innerWidth - 1, Math.max(0, r.left + Math.min(r.width / 2, 40)));
  // Probe the first line only: the middle of a tall paragraph that starts near the bottom edge lies
  // below the viewport, where elementFromPoint answers null and the goal read as covered (R-01).
  const ys = [r.top + 2, r.top + Math.min(r.height, 40) / 2];
  const covered = visible && ys.some((y) => {
    const hit = document.elementFromPoint(x, y);
    return !hit || !(hit === best || best!.contains(hit) || hit.contains(best!));
  });
  return {found: true, visible, covered, top: Math.round(r.top)};
}

// Words of text a reader has in front of them: text nodes inside the viewport.
export function readingLoadProbe(): number {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  let words = 0;
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const text = n.textContent ?? '';
    if (!text.trim()) continue;
    range.selectNodeContents(n);
    const r = range.getBoundingClientRect();
    if (r.width < 1 || r.bottom <= 0 || r.top >= window.innerHeight || r.right <= 0 || r.left >= window.innerWidth) continue;
    const parent = n.parentElement;
    if (parent && getComputedStyle(parent).visibility === 'hidden') continue;
    words += text.trim().split(/\s+/).length;
  }
  return words;
}
