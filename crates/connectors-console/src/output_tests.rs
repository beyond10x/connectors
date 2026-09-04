// The test module for `output.rs`, split out so the module-size fence measures the renderer
// rather than its tests — the same move `catalog-build/src/document.rs` and
// `integration-gitlab/src/backend.rs` made, and included the same way.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_structured_format_carries_its_failure_on_stdout() {
        // The distinction this asserts is the module's reason for existing: a parser reading stdout
        // must find the refusal there, not an empty stream.
        assert!(Format::Json.errors_on_stdout());
        assert!(Format::Yaml.errors_on_stdout());
        assert!(!Format::Text.errors_on_stdout());
        assert!(!Format::Compact.errors_on_stdout());
    }

    #[test]
    fn compact_unwraps_the_one_array_a_list_response_carries() {
        let response = json!({"connections": [
            {"connection_ref": "connection:slack:a", "label": "Slack"},
            {"connection_ref": "connection:slack:b", "label": "Other"},
        ]});
        let rendered = render(Format::Compact, &response).unwrap();
        assert_eq!(rendered.lines().count(), 2);
        assert!(rendered.starts_with("connection_ref=connection:slack:a\tlabel=Slack"));
    }

    #[test]
    fn compact_leaves_a_single_record_as_one_line() {
        let rendered = render(
            Format::Compact,
            &json!({"ready": true, "socket": "/x.sock"}),
        )
        .unwrap();
        assert_eq!(rendered, "ready=true\tsocket=/x.sock");
    }

    #[test]
    fn an_object_with_two_arrays_is_not_unwrapped() {
        // Guessing which array is "the records" would be wrong as often as right.
        let response = json!({"connections": [{"a": 1}], "observations": [{"b": 2}]});
        assert!(unwrap_single_array(&response).is_none());
    }

    #[test]
    fn text_does_not_quote_a_string_a_person_is_reading() {
        let rendered = render(Format::Text, &json!({"label": "Development cluster"})).unwrap();
        assert_eq!(rendered, "label: Development cluster\n");
    }

    #[test]
    fn text_says_none_rather_than_printing_an_empty_bracket() {
        let rendered = render(Format::Text, &json!({"events": []})).unwrap();
        assert_eq!(rendered, "events: (none)\n");
    }

    #[test]
    fn the_result_discriminant_is_stripped_so_compact_can_see_the_records() {
        let tagged =
            json!({"result": "candidate_search", "value": {"candidates": [{"a": 1}, {"a": 2}]}});
        let stripped = payload(tagged);
        assert_eq!(stripped, json!({"candidates": [{"a": 1}, {"a": 2}]}));
        assert_eq!(
            render(Format::Compact, &stripped).unwrap().lines().count(),
            2
        );
    }

    #[test]
    fn a_payload_carrying_its_own_value_field_is_left_alone() {
        // Unwrapping on the presence of `value` alone would eat a real field. Both keys, and only
        // those two keys, is the signature of the wrapper.
        let genuine = json!({"value": 7, "unit": "seconds"});
        assert_eq!(payload(genuine.clone()), genuine);
        let three_keys = json!({"result": "x", "value": 1, "extra": 2});
        assert_eq!(payload(three_keys.clone()), three_keys);
    }

    #[test]
    fn yaml_renders_through_the_maintained_crate() {
        let rendered = render(Format::Yaml, &json!({"ready": true})).unwrap();
        assert_eq!(rendered.trim(), "ready: true");
    }

    /// The shape `doctor` emits: a summary scalar beside one record per check.
    fn doctor_report() -> Value {
        json!({
            "healthy": false,
            "checks": [
                {"check": "configuration", "status": "ok", "detail": "declares slack"},
                {"check": "daemon", "status": "warn", "detail": "not running"},
                {"check": "state-root", "status": "fail", "detail": "belongs to uid 0"},
            ],
        })
    }

    /// The shape `auth status` emits: records whose own fields are lists, one of them a list of
    /// objects. Nothing here may be dropped by any format.
    fn auth_status() -> Value {
        json!({
            "store": "keyring",
            "providers": [
                {
                    "provider": "slack",
                    "instance": "timo-ai",
                    "status": "callable",
                    "credentials": [
                        {"credential": "slack.bot_token", "subject": "app", "state": "stored"},
                        {"credential": "slack.user_token", "subject": "user", "state": "absent"},
                    ],
                    "satisfied_mechanisms": ["slack.bot_token"],
                    "verify": "slack-users-info",
                },
                {
                    "provider": "gitlab",
                    "instance": "default",
                    "status": "not-callable",
                    "credentials": [],
                    "satisfied_mechanisms": [],
                    "verify": null,
                },
            ],
        })
    }

    fn row_holding<'a>(rendered: &'a str, needle: &str) -> &'a str {
        let mut rows = rendered.lines().filter(|line| line.contains(needle));
        let row = rows
            .next()
            .unwrap_or_else(|| panic!("no row carries `{needle}`:\n{rendered}"));
        assert!(
            rows.next().is_none(),
            "`{needle}` is spread over more than one line:\n{rendered}"
        );
        row
    }

    #[test]
    fn text_spends_one_aligned_row_on_each_record() {
        // Six checks used to cost 26 lines: a bare `-` and one line per field. One row per record
        // is the whole story — a label, a header, three rows, and the summary field beside them.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        assert_eq!(rendered.lines().count(), 6, "\n{rendered}");

        let header = rendered
            .lines()
            .nth(1)
            .expect("a header naming the columns");
        for column in ["check", "status", "detail"] {
            assert!(
                header.contains(column),
                "the header drops `{column}`:\n{rendered}"
            );
        }

        // Aligned means every cell of one column starts at the same offset, header included.
        let ok = row_holding(&rendered, "configuration");
        let warn = row_holding(&rendered, "daemon");
        let fail = row_holding(&rendered, "state-root");
        assert_eq!(header.find("status"), ok.find("ok"), "\n{rendered}");
        assert_eq!(header.find("status"), warn.find("warn"), "\n{rendered}");
        assert_eq!(header.find("status"), fail.find("fail"), "\n{rendered}");
        assert_eq!(
            header.find("detail"),
            ok.find("declares slack"),
            "\n{rendered}"
        );
        assert_eq!(
            header.find("detail"),
            warn.find("not running"),
            "\n{rendered}"
        );
        assert_eq!(
            header.find("detail"),
            fail.find("belongs to uid 0"),
            "\n{rendered}"
        );
    }

    #[test]
    fn a_row_shows_its_severity_before_anybody_reads_it() {
        // `doctor.rs` ranks its three states and the renderer used to discard the rank at the last
        // inch, so one `warn` looked exactly like the five `ok` rows above it.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        let marker = |needle: &str| {
            row_holding(&rendered, needle)
                .trim_start()
                .chars()
                .next()
                .expect("a marker")
        };
        assert_eq!(marker("configuration"), '+', "\n{rendered}");
        assert_eq!(marker("daemon"), '!', "\n{rendered}");
        assert_eq!(marker("state-root"), 'x', "\n{rendered}");
    }

    #[test]
    fn severity_survives_a_pipe_because_it_is_not_carried_by_colour() {
        // A colour escape is invisible in a file, in `less`, and under NO_COLOR. The marker is
        // plain ASCII, so redirection cannot lose it.
        let rendered = render(Format::Text, &doctor_report()).unwrap();
        assert!(rendered.is_ascii(), "\n{rendered}");
        assert!(!rendered.contains('\u{1b}'), "\n{rendered}");
    }

    #[test]
    fn a_table_reads_left_to_right_with_the_column_that_runs_long_last() {
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"name": "a", "status": "ok"}, {"name": "bb", "status": "fail"}]}),
        )
        .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "rows:\n",
                "    name  status\n",
                "  + a     ok\n",
                "  x bb    fail\n",
            )
        );
    }

    #[test]
    fn text_keeps_every_field_a_record_carries_including_a_nested_list() {
        let rendered = render(Format::Text, &auth_status()).unwrap();
        // One row per provider, and the store the report was read from beside them.
        assert_eq!(rendered.lines().count(), 5, "\n{rendered}");
        assert!(rendered.contains("store: keyring"), "\n{rendered}");
        for field in [
            "slack.bot_token",
            "stored",
            "slack.user_token",
            "absent",
            "slack-users-info",
            "timo-ai",
        ] {
            assert!(
                rendered.contains(field),
                "`{field}` is missing from the text rendering:\n{rendered}"
            );
        }
    }

    #[test]
    fn compact_keeps_the_scalar_a_list_response_carries_beside_its_records() {
        // `connectors -o compact doctor | grep -c healthy` printed 0: the single array was
        // unwrapped and every scalar beside it was discarded. It rides on every record line rather
        // than taking one of its own, because a line of its own is a line `wc -l` counts as a
        // record and `awk` reads as one.
        let rendered = render(Format::Compact, &doctor_report()).unwrap();
        assert_eq!(rendered.lines().count(), 3, "\n{rendered}");
        for line in rendered.lines() {
            assert!(line.contains("healthy=false"), "\n{rendered}");
            assert!(line.contains("check="), "\n{rendered}");
        }
    }

    #[test]
    fn an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record() {
        // Two contracts collide here and this format's own is the one that wins: a summary on a
        // line of its own is spelled exactly like a record, counted by `wc -l` as a record and
        // read by `awk -F'\t'` as a record. So `compact` — and only `compact` — answers an empty
        // listing with nothing, and `emit` writes no newline for it either. The summary of an
        // empty listing is read in `text`, `json` or `yaml`, all three of which still carry it.
        let empty = json!({"providers": [], "summary": {"listed": 0}});
        assert_eq!(render(Format::Compact, &empty).unwrap(), "");
        assert!(render(Format::Text, &empty).unwrap().contains("listed: 0"));
        assert!(render(Format::Json, &empty)
            .unwrap()
            .contains("\"listed\": 0"));
    }

    #[test]
    fn a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it() {
        // `connect slack` carries one array and it holds strings, so `compact_line` wrote each
        // element bare and the name `events` was never written at all — a token on the line that
        // no `key=value` reader can address, and the acceptance says no format drops a field.
        let rendered = render(
            Format::Compact,
            &json!({"connection_ref": "connection:slack:T1", "events": ["message", "reaction"]}),
        )
        .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "events=message\tconnection_ref=connection:slack:T1\n",
                "events=reaction\tconnection_ref=connection:slack:T1",
            )
        );
    }

    #[test]
    fn compact_keeps_a_field_a_record_carries_below_its_top_level() {
        let rendered = render(Format::Compact, &auth_status()).unwrap();
        assert!(rendered.contains("store=keyring"), "\n{rendered}");
        for pair in [
            "credentials.0.credential=slack.bot_token",
            "credentials.0.state=stored",
            "credentials.1.state=absent",
            "satisfied_mechanisms.0=slack.bot_token",
        ] {
            assert!(
                rendered.contains(pair),
                "`{pair}` is missing from the compact rendering:\n{rendered}"
            );
        }
        // An empty list is still a field the value carries, so it is named rather than omitted.
        assert!(rendered.contains("credentials=(none)"), "\n{rendered}");
    }

    #[test]
    fn the_structured_formats_render_the_bytes_they_rendered_before() {
        // Pinned against what this module emitted before the text work: `-o json` and `-o yaml` are
        // contracts for a caller that parses stdout, and readability is not their reader's problem.
        let report = json!({
            "healthy": true,
            "checks": [{"check": "daemon", "status": "warn", "detail": "not running"}],
        });
        assert_eq!(
            render(Format::Json, &report).unwrap(),
            concat!(
                "{\n",
                "  \"checks\": [\n",
                "    {\n",
                "      \"check\": \"daemon\",\n",
                "      \"detail\": \"not running\",\n",
                "      \"status\": \"warn\"\n",
                "    }\n",
                "  ],\n",
                "  \"healthy\": true\n",
                "}",
            )
        );
        assert_eq!(
            render(Format::Yaml, &report).unwrap(),
            concat!(
                "checks:\n",
                "- check: daemon\n",
                "  detail: not running\n",
                "  status: warn\n",
                "healthy: true\n",
            )
        );
    }

    /// The marker a one-record table shows for a word written under a severity key.
    fn marker_for(word: &str) -> Option<char> {
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"subject": "the-record", "status": word}]}),
        )
        .expect("a text rendering");
        row_holding(&rendered, "the-record")
            .trim_start()
            .chars()
            .next()
    }

    #[test]
    fn a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good() {
        // The renderer is shared with results that come off the wire, whose vocabulary this
        // package does not own. Ranking an unrecognised word as `ok` would be a lie; `?` is not.
        let word = "sasquatch";
        assert_eq!(marker_for(word), Some('?'));
    }

    #[test]
    fn every_status_word_this_package_emits_is_one_the_renderer_can_rank() {
        // The class, checked rather than listed: a marker column is only worth reading if every
        // word that can appear under a severity key has a rank, and a hand-kept list of words
        // would need an adversary to extend it.
        //
        // What this actually guards, exactly, because it reads less than its name suggests: a
        // string literal written **on the same line as** a `"status":`, `"state":` or
        // `"severity":` key, in a non-comment line of a `.rs` file directly under this package's
        // own `src/`. It does not follow a variable (`auth.rs` assigns its credential `state` from
        // one, so `stored`/`absent`/`unavailable` are ranked by hand instead), does not read a
        // `json!` block whose key and value are on different lines, does not recurse into a
        // subdirectory, and does not read any other package — the renderer is shared with results
        // that come off the wire and their vocabulary is out of its reach, which is what the
        // `Unknown` rank exists for. `doctor.rs` covers its own three states end to end.
        let sources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = 0_usize;
        for entry in std::fs::read_dir(&sources).expect("the package's own sources") {
            let path = entry.expect("a source entry").path();
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a readable source file");
            for (offset, line) in source.lines().enumerate() {
                for word in severity_words_written_on(line) {
                    let marker = marker_for(&word);
                    assert!(
                        matches!(marker, Some('+' | '!' | 'x')),
                        "{}:{} writes `{word}` under a severity key and the renderer ranks it \
                         `{marker:?}` rather than ok, warn or fail",
                        path.display(),
                        offset + 1
                    );
                    found += 1;
                }
            }
        }
        assert!(
            found >= 6,
            "the scan ranked {found} words; it has stopped reading the sources"
        );
    }

    #[test]
    fn a_cell_never_carries_a_character_that_breaks_the_row() {
        // The class behind the `doctor` defect: both human formats lay a record out as one line,
        // and nothing upstream promises a value is one line. `toml::de::Error` renders a six-line
        // caret diagram and `doctor` folds it into a `detail`, so a TOML typo — the single most
        // likely reason to run `doctor` — turned one check into six lines with nothing in the
        // marker column. Checked over every control character rather than the newline alone: ESC
        // reaches a cell from any value chosen on the far side of a wire.
        for code in (0..0x20_u32).chain([0x7f, 0x9b]) {
            let control = char::from_u32(code).expect("a control character");
            let value = json!({"rows": [
                {"name": "first", "detail": format!("before{control}after")},
                {"name": "second", "detail": "plain"},
            ]});

            let text = render(Format::Text, &value).unwrap();
            assert_eq!(
                text.lines().count(),
                4,
                "U+{code:04X} in a cell spread one record over more than one row:\n{text:?}"
            );
            assert!(
                !text
                    .chars()
                    .any(|glyph| glyph.is_control() && glyph != '\n'),
                "U+{code:04X} reached the terminal unfolded:\n{text:?}"
            );

            let compact = render(Format::Compact, &value).unwrap();
            assert_eq!(
                compact.lines().count(),
                2,
                "U+{code:04X} in a value broke one record per line:\n{compact:?}"
            );
            for line in compact.lines() {
                // Two fields, so exactly one separator. A tab inside a value would hand an
                // `awk -F'\t'` loop a field that is not there, which is the same defect as the
                // newline wearing different clothes.
                assert_eq!(
                    line.matches('\t').count(),
                    1,
                    "U+{code:04X} in a value became a field separator:\n{line:?}"
                );
                assert!(
                    !line
                        .chars()
                        .any(|glyph| glyph.is_control() && glyph != '\t'),
                    "U+{code:04X} reached the terminal unfolded:\n{line:?}"
                );
            }
        }
    }

    #[test]
    fn the_widest_column_moves_last_even_when_the_record_puts_it_first() {
        // The reordering is the table's headline: without it a free-text column in the middle
        // pushes every column after it off the screen. Asserted on a record that carries the wide
        // column *first*, because a record that already carries it last cannot tell the difference.
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"detail": "a long free-text value", "name": "n", "status": "ok"}]}),
        )
        .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "rows:\n",
                "    name  status  detail\n",
                "  + n     ok      a long free-text value\n",
            )
        );
    }

    #[test]
    fn columns_of_equal_width_keep_the_order_the_record_carries() {
        // Only a column that is *strictly* the widest moves. Two columns of the same width have no
        // reason to be reordered, and reordering one of them would scramble the record's own order
        // for nothing.
        let rendered =
            render(Format::Text, &json!({"rows": [{"a": "xxxx", "b": "yyyy"}]})).unwrap();
        assert_eq!(
            rendered,
            concat!("rows:\n", "    a     b\n", "    xxxx  yyyy\n")
        );
    }

    #[test]
    fn a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank() {
        // A record list is not always uniform. An empty cell would read as "the value is empty";
        // `-` reads as "this record has no such field", which is the true answer and a different
        // one.
        let rendered = render(
            Format::Text,
            &json!({"rows": [
                {"name": "first", "note": "a longer note"},
                {"name": "second"},
            ]}),
        )
        .unwrap();
        assert_eq!(
            rendered,
            concat!(
                "rows:\n",
                "    name    note\n",
                "    first   a longer note\n",
                "    second  -\n",
            )
        );
    }

    #[test]
    fn no_cell_is_ever_empty_so_no_row_can_end_in_whitespace() {
        // The invariant the row writer rests on. Every empty value has a name — `""`, `(none)`,
        // `(empty)`, `null`, `-` — so a record whose every field is empty is still a row on screen
        // rather than a blank line, and the unpadded last column cannot leave trailing space.
        for value in [
            json!(""),
            json!({}),
            json!([]),
            json!(null),
            json!(0),
            json!(false),
            json!([""]),
            json!({"k": ""}),
        ] {
            assert!(!inline(&value).is_empty(), "{value} inlined to nothing");
        }
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"a": "", "b": {}, "c": [], "d": null}]}),
        )
        .unwrap();
        for line in rendered.lines() {
            assert!(!line.trim().is_empty(), "a blank line:\n{rendered:?}");
            assert_eq!(line, line.trim_end(), "trailing space:\n{rendered:?}");
        }
    }

    #[test]
    fn an_unranked_table_still_keeps_the_marker_column() {
        // The marker column is not conditional. A first cell beginning `x ` would otherwise land
        // exactly where a ranked table puts a failure, in the one position a reader has been
        // taught to read as severity.
        let rendered = render(
            Format::Text,
            &json!({"rows": [{"label": "x marks the spot", "target": "connection:prom:a"}]}),
        )
        .unwrap();
        for line in rendered.lines().skip(1) {
            assert_eq!(
                line.chars().nth(2),
                Some(' '),
                "an unranked row put something in the marker column:\n{rendered}"
            );
        }
    }

    #[test]
    fn a_wide_character_cell_keeps_the_column_after_it_aligned() {
        // Widths are terminal columns, not `char`s. A CJK or fullwidth character is two columns, so
        // counting characters displaces every later column on that row — reachable through any
        // value chosen elsewhere, such as a Grafana target's label.
        let rendered = render(
            Format::Text,
            &json!({"rows": [
                {"label": "本番Prometheus", "target": "integration:prometheus"},
                {"label": "staging", "target": "integration:prometheus"},
            ]}),
        )
        .unwrap();
        let offsets: Vec<usize> = rendered
            .lines()
            .filter_map(|line| {
                line.find("integration:prometheus")
                    .map(|byte| display_width(&line[..byte]))
            })
            .collect();
        assert_eq!(offsets.len(), 2, "\n{rendered}");
        assert_eq!(offsets[0], offsets[1], "\n{rendered}");
    }

    #[test]
    fn a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget() {
        // Moving the widest column last cannot fix a wide table: `connectors providers` had eleven
        // columns before the moved one and they came to 196 terminal columns between them, so every
        // row wrapped and nothing after the first column was aligned. Asserted on the real
        // catalogue, which is the table that produced the number.
        for value in [
            crate::providers::run(""),
            json!({"rows": [(0..12)
                .map(|index| (format!("column_{index:02}"), json!("x".repeat(40))))
                .collect::<Map<String, Value>>()]}),
        ] {
            let rendered = render(Format::Text, &value).unwrap();
            let header = rendered.lines().nth(1).expect("a header");
            let last = header
                .rsplit("  ")
                .find(|name| !name.is_empty())
                .expect("a last column");
            let start = header.rfind(last).expect("where it begins");
            assert!(
                display_width(&header[..start]) < TABLE_BUDGET,
                "the last column starts at terminal column {} of a {TABLE_BUDGET}-column \
                 terminal, so none of it is on the line:\n{header}",
                display_width(&header[..start])
            );
        }
    }

    #[test]
    fn the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it() {
        // The module header said "a row is one line and fits a terminal" at a 120-column budget,
        // and the renderer has never made that true: `fit_to_budget` narrows the *leading* columns
        // until the last one begins inside the budget, and the last column is deliberately never
        // cut. So the row is as wide as its final cell. The code is as designed; the sentence was
        // not, and nothing measured it.
        //
        // Both halves are held here, because correcting the sentence alone leaves the next author
        // free to write it again. The width is measured on the real catalogue, which is the table
        // the false claim was written about.
        let rendered = render(Format::Text, &crate::providers::run("")).unwrap();
        let widths: Vec<usize> = rendered.lines().map(display_width).collect();
        let over = widths.iter().filter(|width| **width > TABLE_BUDGET).count();
        assert!(
            over > 0,
            "no line of `connectors providers` is wider than {TABLE_BUDGET} columns, so the \
             renderer now does bound a row and the module header should say so"
        );

        // The module header as one line, so a check on what it says is not a check on where the
        // author wrapped it.
        let source = include_str!("output.rs");
        let header: String = source
            .split("\nuse ")
            .next()
            .unwrap_or(source)
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("//!"))
            .flat_map(str::split_whitespace)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !header.contains("fits a terminal"),
            "output.rs claims a row fits a terminal again. {over} of the {} lines \
             `connectors providers` prints are wider than {TABLE_BUDGET}; the longest is {}",
            widths.len(),
            widths.iter().max().copied().unwrap_or(0)
        );
        assert!(
            header.contains("the last column *begins* inside the budget"),
            "output.rs no longer says what the budget guarantees — that the last column begins \
             inside it — so a reader has only the constant's name to go on"
        );
    }

    #[test]
    fn a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last() {
        // Two claims. A cut cell carries the mark, because a silently truncated value reads as the
        // whole value and gives its reader no reason to go and look at `-o json`. And the cells
        // give way before the column names do: a nameless column makes every cell in it
        // meaningless, so the names are the last thing the layout spends.
        let rendered = render(Format::Text, &crate::providers::run("")).unwrap();
        let header = rendered.lines().nth(1).expect("a header");
        for name in ["authority", "provider", "vendor", "verify", "credentials"] {
            assert!(
                header.contains(name),
                "the layout cut the column name `{name}` while cells still had width:\n{header}"
            );
        }
        assert!(
            rendered.lines().skip(2).any(|line| line.contains(ELISION)),
            "nothing was marked as cut in a table that does not fit:\n{header}"
        );
        // The mark costs one byte as well as one column, so where a reader counting bytes thinks
        // a column begins is where it begins on screen.
        for line in rendered.lines() {
            assert_eq!(
                line.len(),
                display_width(line),
                "the layout put a byte/column skew in a line of its own making:\n{line}"
            );
        }
        // Nothing is lost by it: the structured formats still carry the whole value.
        let json = render(Format::Json, &crate::providers::run("")).unwrap();
        assert!(!json.contains(ELISION));
    }

    #[test]
    fn every_protocol_state_this_renderer_can_be_handed_has_a_rank() {
        use protocol::connection::{ChannelState, ConnectSessionState, ConnectionState};

        // The other half of the vocabulary, and the half this package does not own: these three
        // enums are what a result off the wire puts under a `state` key. The source scan below
        // cannot see them — they arrive as an enum, not as a literal — so they are held to the
        // enums themselves. **Every match here is exhaustive on purpose**: a variant added upstream
        // will not compile until somebody has decided what a reader should see for it, which is the
        // only form of this check that does not rot. `revoked` arriving as `?` is what this is for.
        let connection = |state: ConnectionState| match state {
            ConnectionState::Callable => Severity::Ok,
            // On the way to callable, and reached by doing the next thing in the flow.
            ConnectionState::Created | ConnectionState::Authorized => Severity::Warn,
            ConnectionState::Degraded => Severity::Warn,
            // The authority is gone. Nothing this Connection is asked to do can work.
            ConnectionState::Revoked => Severity::Fail,
        };
        let channel = |state: ChannelState| match state {
            ChannelState::Connected => Severity::Ok,
            ChannelState::Starting | ChannelState::Reconnecting => Severity::Warn,
            ChannelState::Stopped => Severity::Fail,
        };
        let session = |state: ConnectSessionState| match state {
            ConnectSessionState::Completed => Severity::Ok,
            ConnectSessionState::Pending => Severity::Warn,
            ConnectSessionState::Expired | ConnectSessionState::Failed => Severity::Fail,
        };

        let mut ranked = 0_usize;
        for (word, expected) in [
            ConnectionState::Created,
            ConnectionState::Authorized,
            ConnectionState::Callable,
            ConnectionState::Degraded,
            ConnectionState::Revoked,
        ]
        .into_iter()
        .map(|state| {
            (
                serde_json::to_value(state).expect("a wire word"),
                connection(state),
            )
        })
        .chain(
            [
                ChannelState::Starting,
                ChannelState::Connected,
                ChannelState::Reconnecting,
                ChannelState::Stopped,
            ]
            .into_iter()
            .map(|state| {
                (
                    serde_json::to_value(state).expect("a wire word"),
                    channel(state),
                )
            }),
        )
        .chain(
            [
                ConnectSessionState::Pending,
                ConnectSessionState::Completed,
                ConnectSessionState::Expired,
                ConnectSessionState::Failed,
            ]
            .into_iter()
            .map(|state| {
                (
                    serde_json::to_value(state).expect("a wire word"),
                    session(state),
                )
            }),
        ) {
            assert_eq!(severity_of(&word), expected, "the rank of {word}");
            ranked += 1;
        }
        assert_eq!(ranked, 13, "a state went unranked");
    }

    #[test]
    fn two_providers_that_differ_in_their_id_differ_on_screen() {
        // Why the budget is spent on content: with the column *name* paid first, `provider` was
        // squeezed to eight columns, 18 of 65 catalogued ids arrived cut, and four pairs of
        // distinct authorities rendered as the same string. A listing whose identifiers collide is
        // a listing nobody can act on, which is the story's whole point.
        let all = crate::providers::run("");
        let rendered = render(Format::Text, &all).unwrap();
        let mut checked = 0_usize;
        for record in all["providers"].as_array().expect("the records") {
            let id = record["provider"].as_str().expect("an id");
            // Space-delimited: a cell is padded, so a whole id reads as ` id `, and a fragment of
            // one inside a longer cell — `jira` inside `com.atlassian.jira` — does not.
            assert!(
                rendered.contains(&format!(" {id} ")),
                "`{id}` does not appear on screen as a whole id"
            );
            checked += 1;
        }
        assert!(checked > 40, "only {checked} ids were checked");

        // And the names stay apart too: at three columns `reads` and `required_config_fields` are
        // both `re~`, which is the same collision one row up.
        let header = rendered.lines().nth(1).expect("a header");
        let names: Vec<&str> = header.split("  ").filter(|name| !name.is_empty()).collect();
        assert_eq!(
            names.len(),
            names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            "two columns share one name: {names:?}"
        );
    }

    /// Every string literal written as the value of a severity key on one line of Rust source.
    ///
    /// Stops at the next key so `"status": "x", "detail": "y"` yields `x` alone, and keeps reading
    /// past a conditional so `if empty { "a" } else { "b" }` yields both.
    fn severity_words_written_on(line: &str) -> Vec<String> {
        let mut words = Vec::new();
        // A comment emits nothing, and this module's own documentation quotes the shape it reads.
        if line.trim_start().starts_with("//") {
            return words;
        }
        for key in ["\"status\":", "\"state\":", "\"severity\":"] {
            let Some(start) = line.find(key) else {
                continue;
            };
            let mut rest = &line[start + key.len()..];
            while let Some(open) = rest.find('"') {
                let Some(close) = rest[open + 1..].find('"') else {
                    break;
                };
                let word = &rest[open + 1..open + 1 + close];
                rest = &rest[open + close + 2..];
                // A literal immediately followed by `:` is the next key, not a value.
                if rest.trim_start().starts_with(':') {
                    break;
                }
                words.push(word.to_owned());
            }
        }
        words
    }
}
