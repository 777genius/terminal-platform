#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "terminal-platform-capi.h"

static int expect_string_ok(const char *label, TerminalCapiStringResult result, char **out_json) {
  if (result.status != 0 || result.value == NULL) {
    fprintf(stderr, "%s failed with status=%d payload=%s\n", label, (int)result.status,
            result.value != NULL ? result.value : "<null>");
    if (result.value != NULL) {
      terminal_capi_string_free(result.value);
    }
    return 0;
  }

  *out_json = result.value;
  return 1;
}

static int expect_client_ok(const char *label, TerminalCapiClientResult result,
                            TerminalCapiClientHandle **out_client) {
  if (result.status != 0 || result.client == NULL) {
    fprintf(stderr, "%s failed with status=%d payload=%s\n", label, (int)result.status,
            result.error != NULL ? result.error : "<null>");
    if (result.error != NULL) {
      terminal_capi_string_free(result.error);
    }
    return 0;
  }

  *out_client = result.client;
  return 1;
}

static int expect_subscription_ok(const char *label, TerminalCapiSubscriptionResult result,
                                  TerminalCapiSubscriptionHandle **out_subscription) {
  if (result.status != 0 || result.subscription == NULL) {
    fprintf(stderr, "%s failed with status=%d payload=%s\n", label, (int)result.status,
            result.error != NULL ? result.error : "<null>");
    if (result.error != NULL) {
      terminal_capi_string_free(result.error);
    }
    return 0;
  }

  *out_subscription = result.subscription;
  return 1;
}

static int json_contains(const char *json, const char *needle) { return strstr(json, needle) != NULL; }

static int write_text_file(const char *path, const char *content) {
  FILE *file = NULL;

  if (path == NULL || path[0] == '\0') {
    fprintf(stderr, "missing file path for write_text_file\n");
    return 0;
  }

  file = fopen(path, "w");
  if (file == NULL) {
    fprintf(stderr, "failed to open file for write: %s\n", path);
    return 0;
  }

  if (fputs(content, file) == EOF) {
    fprintf(stderr, "failed to write file: %s\n", path);
    fclose(file);
    return 0;
  }

  if (fclose(file) != 0) {
    fprintf(stderr, "failed to close file after write: %s\n", path);
    return 0;
  }

  return 1;
}

static int wait_for_file(const char *path, const char *label) {
  int attempt = 0;

  if (path == NULL || path[0] == '\0') {
    fprintf(stderr, "missing path while waiting for %s\n", label);
    return 0;
  }

  for (attempt = 0; attempt < 600; ++attempt) {
    FILE *file = fopen(path, "r");
    if (file != NULL) {
      fclose(file);
      return 1;
    }
    usleep(100000);
  }

  fprintf(stderr, "timed out waiting for %s at %s\n", label, path);
  return 0;
}

static int extract_json_string(const char *json, const char *key, char *out, size_t out_len) {
  char pattern[128];
  const char *start = NULL;
  const char *end = NULL;
  size_t length = 0;

  if (snprintf(pattern, sizeof(pattern), "\"%s\":\"", key) >= (int)sizeof(pattern)) {
    fprintf(stderr, "json key pattern for %s is too large\n", key);
    return 0;
  }

  start = strstr(json, pattern);
  if (start == NULL) {
    fprintf(stderr, "missing json key %s in payload: %s\n", key, json);
    return 0;
  }

  start += strlen(pattern);
  end = strchr(start, '"');
  if (end == NULL) {
    fprintf(stderr, "unterminated json string for key %s in payload: %s\n", key, json);
    return 0;
  }

  length = (size_t)(end - start);
  if (length + 1 > out_len) {
    fprintf(stderr, "buffer too small for key %s\n", key);
    return 0;
  }

  memcpy(out, start, length);
  out[length] = '\0';
  return 1;
}

static int extract_json_object(const char *json, const char *key, char *out, size_t out_len) {
  char pattern[128];
  const char *start = NULL;
  const char *cursor = NULL;
  const char *end = NULL;
  int depth = 0;
  int in_string = 0;
  int escaped = 0;
  size_t length = 0;

  if (snprintf(pattern, sizeof(pattern), "\"%s\":", key) >= (int)sizeof(pattern)) {
    fprintf(stderr, "json key pattern for %s is too large\n", key);
    return 0;
  }

  start = strstr(json, pattern);
  if (start == NULL) {
    fprintf(stderr, "missing json object key %s in payload: %s\n", key, json);
    return 0;
  }

  start += strlen(pattern);
  while (*start == ' ' || *start == '\n' || *start == '\r' || *start == '\t') {
    start++;
  }

  if (*start != '{') {
    fprintf(stderr, "json key %s does not point to an object in payload: %s\n", key, json);
    return 0;
  }

  for (cursor = start; *cursor != '\0'; ++cursor) {
    char ch = *cursor;

    if (in_string) {
      if (escaped) {
        escaped = 0;
      } else if (ch == '\\') {
        escaped = 1;
      } else if (ch == '"') {
        in_string = 0;
      }
      continue;
    }

    if (ch == '"') {
      in_string = 1;
      continue;
    }

    if (ch == '{') {
      depth++;
      continue;
    }

    if (ch == '}') {
      depth--;
      if (depth == 0) {
        end = cursor + 1;
        break;
      }
    }
  }

  if (end == NULL) {
    fprintf(stderr, "unterminated json object for key %s in payload: %s\n", key, json);
    return 0;
  }

  length = (size_t)(end - start);
  if (length + 1 > out_len) {
    fprintf(stderr, "buffer too small for object key %s\n", key);
    return 0;
  }

  memcpy(out, start, length);
  out[length] = '\0';
  return 1;
}

static int wait_for_event_with_substring(const char *label,
                                         TerminalCapiSubscriptionHandle *subscription,
                                         const char *expected_kind, const char *needle,
                                         char **out_json) {
  int attempt = 0;

  for (attempt = 0; attempt < 16; ++attempt) {
    char *json = NULL;
    if (!expect_string_ok(label, terminal_capi_subscription_next_event_json(subscription), &json)) {
      return 0;
    }

    if (json_contains(json, expected_kind) && json_contains(json, needle)) {
      *out_json = json;
      return 1;
    }

    terminal_capi_string_free(json);
  }

  fprintf(stderr, "%s never observed %s with %s\n", label, expected_kind, needle);
  return 0;
}

static int wait_for_subscription_close(const char *label,
                                       TerminalCapiSubscriptionHandle *subscription) {
  int attempt = 0;

  for (attempt = 0; attempt < 32; ++attempt) {
    char *json = NULL;
    if (!expect_string_ok(label, terminal_capi_subscription_next_event_json(subscription), &json)) {
      return 0;
    }

    if (strcmp(json, "null") == 0) {
      terminal_capi_string_free(json);
      return 1;
    }

    terminal_capi_string_free(json);
  }

  fprintf(stderr, "%s never observed null subscription close\n", label);
  return 0;
}

int main(int argc, char **argv) {
  TerminalCapiClientHandle *client = NULL;
  TerminalCapiSubscriptionHandle *topology_subscription = NULL;
  TerminalCapiSubscriptionHandle *pane_subscription = NULL;
  char session_id[128];
  char pane_id[128];
  char pane_subscription_spec[256];
  char route_json[512];
  char *json = NULL;
  char *event = NULL;
  const char *mode = "native";
  int ok = 0;

  if (argc != 3 && argc != 4) {
    fprintf(stderr, "usage: %s <namespaced|filesystem> <address> [native|tmux|shutdown|restart]\n",
            argv[0]);
    return 1;
  }

  if (argc == 4) {
    mode = argv[3];
  }

  if (strcmp(argv[1], "namespaced") == 0) {
    if (!expect_client_ok("new_from_namespaced_address",
                          terminal_capi_client_new_from_namespaced_address(argv[2]), &client)) {
      return 1;
    }
  } else if (strcmp(argv[1], "filesystem") == 0) {
    if (!expect_client_ok("new_from_filesystem_path",
                          terminal_capi_client_new_from_filesystem_path(argv[2]), &client)) {
      return 1;
    }
  } else {
    fprintf(stderr, "unsupported address kind %s\n", argv[1]);
    return 1;
  }

  if (!expect_string_ok("binding_version", terminal_capi_client_binding_version_json(client), &json)) {
    goto cleanup;
  }
  if (!json_contains(json, "\"binding_version\"")) {
    fprintf(stderr, "binding_version payload missing expected field: %s\n", json);
    goto cleanup;
  }
  terminal_capi_string_free(json);
  json = NULL;

  if (!expect_string_ok("handshake_info", terminal_capi_client_handshake_info_json(client), &json)) {
    goto cleanup;
  }
  if (!json_contains(json, "\"can_use\":true")) {
    fprintf(stderr, "handshake payload does not report can_use=true: %s\n", json);
    goto cleanup;
  }
  terminal_capi_string_free(json);
  json = NULL;

  if (strcmp(mode, "native") == 0 || strcmp(mode, "shutdown") == 0 ||
      strcmp(mode, "restart") == 0) {
    if (!expect_string_ok("backend_capabilities",
                          terminal_capi_client_backend_capabilities_json(client, "native"),
                          &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"backend\":\"native\"") ||
        !json_contains(json, "\"explicit_session_save\":true")) {
      fprintf(stderr,
              "backend_capabilities payload is missing expected native capabilities: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok(
            "create_native_session",
            terminal_capi_client_create_native_session_json(
                client,
                "{\"title\":\"c-consumer\",\"launch\":{\"program\":\"/bin/sh\",\"args\":[\"-lc\","
                "\"printf 'ready\\\\n'; exec cat\"]}}"),
            &json)) {
      goto cleanup;
    }
    if (!extract_json_string(json, "session_id", session_id, sizeof(session_id))) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("list_sessions", terminal_capi_client_list_sessions_json(client), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, session_id)) {
      fprintf(stderr, "list_sessions payload does not contain session_id %s: %s\n", session_id,
              json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("attach_session",
                          terminal_capi_client_attach_session_json(client, session_id), &json)) {
      goto cleanup;
    }
    if (!extract_json_string(json, "pane_id", pane_id, sizeof(pane_id))) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_subscription_ok("open_topology_subscription",
                                terminal_capi_client_open_subscription(
                                    client, session_id, "{\"kind\":\"session_topology\"}"),
                                &topology_subscription)) {
      goto cleanup;
    }

    if (!expect_string_ok("topology_subscription_meta",
                          terminal_capi_subscription_meta_json(topology_subscription), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"subscription_id\":\"")) {
      fprintf(stderr, "topology subscription meta is missing subscription_id: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("topology_subscription_initial",
                          terminal_capi_subscription_next_event_json(topology_subscription),
                          &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"kind\":\"topology_snapshot\"")) {
      fprintf(stderr, "topology subscription initial event is unexpected: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (snprintf(pane_subscription_spec, sizeof(pane_subscription_spec),
                 "{\"kind\":\"pane_surface\",\"pane_id\":\"%s\"}", pane_id) >=
        (int)sizeof(pane_subscription_spec)) {
      fprintf(stderr, "pane subscription spec is too large\n");
      goto cleanup;
    }

    if (!expect_subscription_ok("open_pane_subscription",
                                terminal_capi_client_open_subscription(client, session_id,
                                                                       pane_subscription_spec),
                                &pane_subscription)) {
      goto cleanup;
    }

    if (!expect_string_ok("pane_subscription_initial",
                          terminal_capi_subscription_next_event_json(pane_subscription), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"kind\":\"screen_delta\"") ||
        !json_contains(json, "\"full_replace\":")) {
      fprintf(stderr, "pane subscription initial event is unexpected: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("dispatch_new_tab",
                          terminal_capi_client_dispatch_mux_command_json(
                              client, session_id, "{\"kind\":\"new_tab\",\"title\":\"logs\"}"),
                          &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"changed\":true")) {
      fprintf(stderr, "dispatch new_tab did not report changed=true: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!wait_for_event_with_substring("wait_topology_update", topology_subscription,
                                       "\"kind\":\"topology_snapshot\"", "\"title\":\"logs\"",
                                       &event)) {
      goto cleanup;
    }
    terminal_capi_string_free(event);
    event = NULL;

    {
      char input_command[384];
      if (snprintf(input_command, sizeof(input_command),
                   "{\"kind\":\"send_input\",\"pane_id\":\"%s\",\"data\":\"ffi c consumer "
                   "input\\r\"}",
                   pane_id) >= (int)sizeof(input_command)) {
        fprintf(stderr, "input command buffer is too small\n");
        goto cleanup;
      }

      if (!expect_string_ok("dispatch_send_input",
                            terminal_capi_client_dispatch_mux_command_json(client, session_id,
                                                                           input_command),
                            &json)) {
        goto cleanup;
      }
    }
    if (!json_contains(json, "\"changed\":false")) {
      fprintf(stderr, "dispatch send_input did not report changed=false: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!wait_for_event_with_substring("wait_pane_update", pane_subscription,
                                       "\"kind\":\"screen_delta\"", "ffi c consumer input",
                                       &event)) {
      goto cleanup;
    }
    terminal_capi_string_free(event);
    event = NULL;

    if (strcmp(mode, "shutdown") == 0) {
      const char *ready_file = getenv("TERMINAL_CAPI_READY_FILE");

      if (!write_text_file(ready_file, "ready\n")) {
        goto cleanup;
      }

      if (!wait_for_subscription_close("topology_subscription_shutdown", topology_subscription)) {
        goto cleanup;
      }
      if (!wait_for_subscription_close("pane_subscription_shutdown", pane_subscription)) {
        goto cleanup;
      }

      ok = 1;
      goto cleanup;
    }

    if (strcmp(mode, "restart") == 0) {
      const char *initial_ready_file = getenv("TERMINAL_CAPI_INITIAL_READY_FILE");
      const char *stale_ready_file = getenv("TERMINAL_CAPI_STALE_READY_FILE");
      const char *restart_file = getenv("TERMINAL_CAPI_RESTART_FILE");
      int stale_error_observed = 0;

      if (!write_text_file(initial_ready_file, "ready\n")) {
        goto cleanup;
      }

      while (!stale_error_observed) {
        TerminalCapiStringResult handshake_result =
            terminal_capi_client_handshake_info_json(client);
        if (handshake_result.status == 0 && handshake_result.value != NULL) {
          terminal_capi_string_free(handshake_result.value);
          usleep(100000);
          continue;
        }

        stale_error_observed = 1;
        if (handshake_result.value != NULL) {
          terminal_capi_string_free(handshake_result.value);
        }
      }

      if (!write_text_file(stale_ready_file, "stale\n")) {
        goto cleanup;
      }
      if (!wait_for_file(restart_file, "daemon restart signal")) {
        goto cleanup;
      }

      while (1) {
        TerminalCapiStringResult handshake_result =
            terminal_capi_client_handshake_info_json(client);
        if (handshake_result.status != 0 || handshake_result.value == NULL) {
          if (handshake_result.value != NULL) {
            terminal_capi_string_free(handshake_result.value);
          }
          usleep(100000);
          continue;
        }

        if (!json_contains(handshake_result.value, "\"can_use\":true")) {
          fprintf(stderr, "restarted handshake did not report can_use=true: %s\n",
                  handshake_result.value);
          terminal_capi_string_free(handshake_result.value);
          goto cleanup;
        }
        terminal_capi_string_free(handshake_result.value);
        break;
      }

      if (!expect_string_ok(
              "create_native_session_after_restart",
              terminal_capi_client_create_native_session_json(
                  client,
                  "{\"title\":\"c-consumer-restart\",\"launch\":{\"program\":\"/bin/sh\","
                  "\"args\":[\"-lc\",\"printf 'restart-ready\\\\n'; exec cat\"]}}"),
              &json)) {
        goto cleanup;
      }
      terminal_capi_string_free(json);
      json = NULL;

      printf("{\"stale_error_observed\":true,\"recovered\":true}\n");
      fflush(stdout);
      ok = 1;
      goto cleanup;
    }

    if (!expect_string_ok("close_topology_subscription",
                          terminal_capi_subscription_close(topology_subscription), &json)) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("close_pane_subscription",
                          terminal_capi_subscription_close(pane_subscription), &json)) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    ok = 1;
  } else if (strcmp(mode, "tmux") == 0) {
    if (!expect_string_ok("backend_capabilities",
                          terminal_capi_client_backend_capabilities_json(client, "tmux"), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"backend\":\"tmux\"")) {
      fprintf(stderr, "backend_capabilities payload is missing tmux backend: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("discover_sessions",
                          terminal_capi_client_discover_sessions_json(client, "tmux"), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"backend\":\"tmux\"")) {
      fprintf(stderr, "discover_sessions payload is missing tmux route: %s\n", json);
      goto cleanup;
    }
    if (!extract_json_object(json, "route", route_json, sizeof(route_json))) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("import_session",
                          terminal_capi_client_import_session_json(client, route_json, NULL),
                          &json)) {
      goto cleanup;
    }
    if (!extract_json_string(json, "session_id", session_id, sizeof(session_id))) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("topology_snapshot",
                          terminal_capi_client_topology_snapshot_json(client, session_id), &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "\"backend_kind\":\"tmux\"")) {
      fprintf(stderr, "topology_snapshot payload is missing tmux backend kind: %s\n", json);
      goto cleanup;
    }
    if (!extract_json_string(json, "pane_id", pane_id, sizeof(pane_id))) {
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    if (!expect_string_ok("screen_snapshot",
                          terminal_capi_client_screen_snapshot_json(client, session_id, pane_id),
                          &json)) {
      goto cleanup;
    }
    if (!json_contains(json, "hello from tmux")) {
      fprintf(stderr, "screen_snapshot payload did not contain tmux shell output: %s\n", json);
      goto cleanup;
    }
    terminal_capi_string_free(json);
    json = NULL;

    ok = 1;
  } else {
    fprintf(stderr, "unsupported mode %s\n", mode);
    goto cleanup;
  }

cleanup:
  if (event != NULL) {
    terminal_capi_string_free(event);
  }
  if (json != NULL) {
    terminal_capi_string_free(json);
  }
  if (topology_subscription != NULL) {
    terminal_capi_subscription_free(topology_subscription);
  }
  if (pane_subscription != NULL) {
    terminal_capi_subscription_free(pane_subscription);
  }
  if (client != NULL) {
    terminal_capi_client_free(client);
  }

  return ok ? 0 : 1;
}
