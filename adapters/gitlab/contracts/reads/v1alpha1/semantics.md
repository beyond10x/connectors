# gitlab implemented read profiles/v1alpha1

This adapter owns the native behavior of its existing local implementation.
Shared wire envelopes and admission remain in
[service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md).
This relocation changes no runtime or selected codec.

GitLab: project metadata, paginated project issues, repository file content/metadata.
Project allowlists apply before dispatch. Pagination preserves provider continuation
and has cursors bound to exact operation/input/instance/config identity and expiry.
GitLab source facts: https://docs.gitlab.com/api/projects/,
https://docs.gitlab.com/api/issues/, https://docs.gitlab.com/api/repository_files/,
https://docs.gitlab.com/api/rest/ and
https://docs.gitlab.com/api/rest/authentication/. Source access: 2026-09-08.
