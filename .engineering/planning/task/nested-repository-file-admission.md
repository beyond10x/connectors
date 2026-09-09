---
format: aep.planning-md/1
id: task:nested-repository-file-admission
kind: task
status: draft
title: Admit nested repository file paths through the local invocation contract
revision: 1
---
# Nested GitLab repository-file input rejected by live local operation

Consumer contract: a local read-only agent must inspect a file returned by gitlab-repository-tree-list at the same exact commit using gitlab-repository-file-get. Observed with connectors 0.7.0, protocol v3, configured local daemon.

The fresh operation description requires project_id:number, file_path:string and ref:string. A repository-relative nested path returned by tree-list (ordinary directory names, no dot segments, no backslashes) plus its exact 40-character commit was rejected by operation invoke with exit 1 and {"error":{"code":"invalid_input","message":"the operation input was invalid","retriable":false},"target":"local"}. A root README.md file from the same project and exact commit succeeded through the same operation, connection and description lease. Tree listing and branch resolution also succeeded. No fallback transport was used after refusal.

Synthetic reproduction to add: a permitted fixture project containing README.md and sdk/src/client.ts; describe then invoke file-get for both at one commit. Both must resolve through the catalogued path; the nested value must be encoded as one GitLab URL segment. Reject traversal and changed-root inputs. Cover the actual local daemon invocation admission path, not only integration-gitlab helper tests. Fresh schema must describe all caller constraints; preserve structured error details without credentials.

Existing source already has integration-gitlab/src/repository_file.rs with a special repository-relative path encoder and tests. This report does not establish which live dispatch/admission layer bypasses or rejects it. Diagnose the installed release path before assuming that helper needs reimplementation.

Owner: Connectors maintainers. Consumer session only files this scoped request; it does not implement, merge or release Connectors. Private provider identity, customer content and repository paths remain outside this public artifact. No transport or credential-context changes are authorized by the refusal.
