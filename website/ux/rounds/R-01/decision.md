# R-01 decision: Add a site-local search to the Connectors site now, ahead of Docs System?

Finding: UXF-001. Target metric: gate 2 `search` absent, now 171 of 171 pages at both gating
viewports; expected 0 with search.
Yes: add a local, build-time search to this site (a Docusaurus search plugin as a new
dependency, its box in the navbar), reversing the design record's rule in
`docs/website-design.md` ("Search returns when the shared shell offers it"); shot:
shots/decision-no.png shows the navbar as it is, a yes shot needs the plugin built and is not taken.
No: the navbar stays without search; UXF-001 is passed to Docs System, and the gate holds `search`
as a known loss (`knownAbsent` in `ux/adapter.ts`) that fails the gate as soon as the shell ships
search. Shot: shots/decision-no.png.
Default if unanswered by the next round: no, because the design record assigns search to the shared
shell, every beyond10x site would need the same answer, and a new dependency is a change the
operator decides.
Answer:
