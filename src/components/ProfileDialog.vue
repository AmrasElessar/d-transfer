<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import { useProfilesStore } from "@/stores/profiles";
import FieldSegmented from "@/components/widgets/FieldSegmented.vue";
import type {
  AdapterCapabilities,
  AuthMethod,
  ConnectionProfile,
  ProfileProtocol,
} from "@/types";

const { t } = useI18n();
const profilesStore = useProfilesStore();

const props = defineProps<{
  open: boolean;
  /** `null` = create mode; otherwise edit mode bound to this profile. */
  profile: ConnectionProfile | null;
}>();
const emit = defineEmits<{ close: [] }>();

interface FormState {
  name: string;
  protocol: ProfileProtocol;
  host: string;
  port: number | null;
  username: string;
  remoteRoot: string;
  localRoot: string;
  authMethod: AuthMethod;
  /** UI-only field; backend'e gönderilirken `secret` parametresine map'lenir. */
  password: string;
  /** `secret`'i mutate etmeyi açıkça istedi mi? edit mode'da default false —
   *  böylece sadece "Yeni profil" akışında password yazılır, edit'te dokunulmaz. */
  passwordTouched: boolean;
}

function blankForm(): FormState {
  return {
    name: "",
    protocol: "local",
    host: "",
    port: null,
    username: "",
    remoteRoot: "",
    localRoot: "",
    authMethod: "none",
    password: "",
    passwordTouched: false,
  };
}

function fromProfile(p: ConnectionProfile): FormState {
  return {
    name: p.name,
    protocol: p.protocol,
    host: p.host ?? "",
    port: p.port,
    username: p.username ?? "",
    remoteRoot: p.remoteRoot ?? "",
    localRoot: p.localRoot ?? "",
    authMethod: p.authMethod,
    password: "",
    passwordTouched: false,
  };
}

const form = reactive<FormState>(blankForm());
const showPassword = ref(false);
const saving = ref(false);
const testing = ref(false);
const testResult = ref<
  | { kind: "success"; caps: AdapterCapabilities }
  | { kind: "error"; message: string }
  | null
>(null);
const submitError = ref<string | null>(null);

const isEdit = computed(() => props.profile !== null);

// Default port hint per protocol — UI ergonomics. Backend tarafı `null`'ı
// kabul ediyor (varsayılan adapter'ın işi); UI yine de boş alanı görünür yapar.
const DEFAULT_PORTS: Record<ProfileProtocol, number | null> = {
  local: null,
  sftp: 22,
  s3: 443,
  webdav: 443,
};

watch(
  () => [props.open, props.profile],
  () => {
    if (!props.open) return;
    if (props.profile) {
      Object.assign(form, fromProfile(props.profile));
    } else {
      Object.assign(form, blankForm());
    }
    testResult.value = null;
    submitError.value = null;
    showPassword.value = false;
  },
  { immediate: true },
);

function setProtocol(p: ProfileProtocol) {
  form.protocol = p;
  if (form.port === null && DEFAULT_PORTS[p] !== null) {
    form.port = DEFAULT_PORTS[p];
  }
  if (p === "local") {
    form.authMethod = "none";
    form.host = "";
    form.port = null;
  } else if (form.authMethod === "none") {
    form.authMethod = "password";
  }
}

function markPasswordTouched() {
  form.passwordTouched = true;
}

async function pickLocalRoot() {
  const selected = await openDialog({ directory: true, multiple: false });
  if (typeof selected === "string") {
    form.localRoot = selected;
  }
}

// Validation: name required; sftp/webdav/s3 need host + username; local needs localRoot.
const validationError = computed<string | null>(() => {
  if (!form.name.trim()) return t("profiles.errors.nameRequired");
  if (form.protocol === "local") {
    if (!form.localRoot.trim()) return t("profiles.errors.localRootRequired");
  } else {
    if (!form.host.trim()) return t("profiles.errors.hostRequired");
    if (form.protocol === "sftp" && !form.username.trim()) {
      return t("profiles.errors.usernameRequired");
    }
  }
  return null;
});

function buildProfile(): ConnectionProfile {
  const now = new Date().toISOString();
  const base = props.profile;
  return {
    id: base?.id ?? crypto.randomUUID(),
    name: form.name.trim(),
    protocol: form.protocol,
    host: form.protocol === "local" ? null : form.host.trim() || null,
    port: form.protocol === "local" ? null : form.port,
    username: form.username.trim() || null,
    remoteRoot: form.remoteRoot.trim() || null,
    localRoot: form.protocol === "local" ? form.localRoot.trim() || null : null,
    authMethod: form.authMethod,
    optionsJson: base?.optionsJson ?? "{}",
    createdAt: base?.createdAt ?? now,
    updatedAt: now,
  };
}

/**
 * Secret parametresinin semantiği:
 * - Create mode: password alanı doluysa o değeri gönder, boşsa undefined (yazma).
 * - Edit mode: kullanıcı password alanına dokunmadıysa undefined (eski sırrı koru).
 *   Dokundu ve boş → "" göndererek eski sırrı sil.
 *   Dokundu ve dolu → yeni değer.
 */
function secretToSend(): string | undefined {
  if (!isEdit.value) {
    return form.password.length > 0 ? form.password : undefined;
  }
  if (!form.passwordTouched) return undefined;
  return form.password;
}

async function runTest() {
  testResult.value = null;
  testing.value = true;
  try {
    const draft = buildProfile();
    const caps = await profilesStore.testConnection(draft, secretToSend());
    testResult.value = { kind: "success", caps };
  } catch (err) {
    testResult.value = { kind: "error", message: extractMessage(err) };
  } finally {
    testing.value = false;
  }
}

async function submit() {
  if (validationError.value) {
    submitError.value = validationError.value;
    return;
  }
  submitError.value = null;
  saving.value = true;
  try {
    const draft = buildProfile();
    if (isEdit.value) {
      await profilesStore.update(draft, secretToSend());
    } else {
      await profilesStore.create(draft, secretToSend());
    }
    emit("close");
  } catch (err) {
    submitError.value = extractMessage(err);
  } finally {
    saving.value = false;
  }
}

const protocolOptions = computed<{ value: ProfileProtocol; label: string }[]>(
  () =>
    (["local", "sftp", "s3", "webdav"] as const).map((v) => ({
      value: v,
      label: t(`profiles.protocols.${v}`),
    })),
);

const authOptions = computed<{ value: AuthMethod; label: string }[]>(() =>
  (["none", "password", "publicKey"] as const).map((v) => ({
    value: v,
    label: t(`profiles.auth.${v}`),
  })),
);

function extractMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

function summarizeCaps(caps: AdapterCapabilities): string {
  const parts: string[] = [];
  if (caps.supportsResume) parts.push("resume");
  if (caps.supportsByteRange) parts.push("range");
  if (caps.supportsRemoteChecksum) parts.push("checksum");
  if (caps.supportsServerSideRename) parts.push("rename");
  if (caps.supportsSymlinks) parts.push("symlinks");
  if (caps.supportsMultipart) parts.push("multipart");
  parts.push(`${caps.maxParallelSessions}-parallel`);
  return parts.join(" · ");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && props.open) emit("close");
}
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-surface-base/70 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    :aria-label="isEdit ? t('profiles.edit') : t('profiles.new')"
    @click.self="emit('close')"
    @keydown="onKeydown"
  >
    <div
      class="flex max-h-[85vh] w-[560px] max-w-[92vw] flex-col rounded-md border border-border-default bg-surface-overlay text-fg-default shadow-2xl outline-none"
      tabindex="-1"
    >
      <header class="flex shrink-0 items-center justify-between border-b border-border-muted px-4 py-3">
        <h2 class="text-sm font-medium">
          {{ isEdit ? t("profiles.edit") : t("profiles.new") }}
        </h2>
        <button
          type="button"
          class="rounded-md border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
          :aria-label="t('settings.actions.close')"
          @click="emit('close')"
        >
          ×
        </button>
      </header>

      <div class="flex-1 overflow-y-auto px-4 py-3 text-sm">
        <!-- Name -->
        <div class="mb-3 grid grid-cols-[140px_1fr] items-center gap-3">
          <label for="prof-name" class="text-fg-default">{{ t("profiles.fields.name") }}</label>
          <input
            id="prof-name"
            v-model="form.name"
            type="text"
            autocomplete="off"
            class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
          />
        </div>

        <!-- Protocol -->
        <FieldSegmented
          :label="t('profiles.fields.protocol')"
          :options="protocolOptions"
          :model-value="form.protocol"
          :label-width="140"
          @update="setProtocol"
        />

        <!-- Local root (local only) -->
        <div v-if="form.protocol === 'local'" class="mb-3 grid grid-cols-[140px_1fr] items-start gap-3">
          <label for="prof-localroot" class="text-fg-default">{{ t("profiles.fields.localRoot") }}</label>
          <div class="flex flex-col gap-1">
            <input
              id="prof-localroot"
              v-model="form.localRoot"
              type="text"
              autocomplete="off"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
            />
            <button
              type="button"
              class="self-start rounded border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
              @click="pickLocalRoot"
            >
              {{ t("settings.actions.chooseDir") }}
            </button>
          </div>
        </div>

        <!-- Host + port (non-local) -->
        <template v-else>
          <div class="mb-3 grid grid-cols-[140px_1fr_80px] items-center gap-3">
            <label for="prof-host" class="text-fg-default">{{ t("profiles.fields.host") }}</label>
            <input
              id="prof-host"
              v-model="form.host"
              type="text"
              autocomplete="off"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
            />
            <input
              v-model.number="form.port"
              type="number"
              :placeholder="String(DEFAULT_PORTS[form.protocol] ?? '')"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 text-right font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
              :aria-label="t('profiles.fields.port')"
            />
          </div>

          <div class="mb-3 grid grid-cols-[140px_1fr] items-center gap-3">
            <label for="prof-user" class="text-fg-default">{{ t("profiles.fields.username") }}</label>
            <input
              id="prof-user"
              v-model="form.username"
              type="text"
              autocomplete="off"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
            />
          </div>

          <FieldSegmented
            :label="t('profiles.fields.authMethod')"
            :options="authOptions"
            :model-value="form.authMethod"
            :label-width="140"
            @update="(v: AuthMethod) => (form.authMethod = v)"
          />

          <div v-if="form.authMethod === 'password'" class="mb-3 grid grid-cols-[140px_1fr_auto] items-center gap-3">
            <label for="prof-pw" class="text-fg-default">{{ t("profiles.fields.password") }}</label>
            <input
              id="prof-pw"
              v-model="form.password"
              :type="showPassword ? 'text' : 'password'"
              autocomplete="new-password"
              :placeholder="isEdit ? '••••••••' : ''"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
              @input="markPasswordTouched"
            />
            <button
              type="button"
              class="rounded border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default"
              @click="showPassword = !showPassword"
            >
              {{ showPassword ? "🙈" : "👁" }}
            </button>
          </div>

          <div class="mb-3 grid grid-cols-[140px_1fr] items-center gap-3">
            <label for="prof-remoteroot" class="text-fg-default">{{ t("profiles.fields.remoteRoot") }}</label>
            <input
              id="prof-remoteroot"
              v-model="form.remoteRoot"
              type="text"
              autocomplete="off"
              placeholder="/"
              class="rounded border border-border-muted bg-surface-base px-2 py-1 font-mono text-xs text-fg-default focus:border-accent-default focus:outline-none"
            />
          </div>
        </template>

        <!-- Test result inline -->
        <div
          v-if="testResult"
          class="mt-3 rounded border px-2 py-1 text-xs"
          :class="testResult.kind === 'success'
            ? 'border-status-success/40 bg-status-success/10 text-status-success'
            : 'border-status-danger/40 bg-status-danger/10 text-status-danger'"
        >
          <span v-if="testResult.kind === 'success'">
            {{ t("profiles.actions.testSuccess") }} —
            <span class="font-mono">{{ summarizeCaps(testResult.caps) }}</span>
          </span>
          <span v-else>
            {{ t("profiles.actions.testFailed") }}: {{ testResult.message }}
          </span>
        </div>

        <div
          v-if="submitError"
          class="mt-3 rounded border border-status-danger/40 bg-status-danger/10 px-2 py-1 text-xs text-status-danger"
        >
          {{ submitError }}
        </div>
        <div
          v-else-if="validationError"
          class="mt-3 rounded border border-status-warning/40 bg-status-warning/10 px-2 py-1 text-xs text-status-warning"
        >
          {{ validationError }}
        </div>
      </div>

      <footer class="flex shrink-0 items-center justify-between border-t border-border-muted px-4 py-2 text-xs text-fg-muted">
        <button
          type="button"
          :disabled="testing || saving"
          class="rounded border border-border-muted px-2 py-1 text-xs text-fg-muted hover:bg-surface-overlay hover:text-fg-default disabled:cursor-not-allowed disabled:opacity-50"
          @click="runTest"
        >
          {{ testing ? t("profiles.actions.testing") : t("profiles.actions.test") }}
        </button>
        <div class="flex gap-2">
          <button
            type="button"
            class="rounded border border-border-muted px-2 py-1 text-xs hover:bg-surface-overlay hover:text-fg-default"
            @click="emit('close')"
          >
            {{ t("profiles.actions.cancel") }}
          </button>
          <button
            type="button"
            :disabled="saving || validationError !== null"
            class="rounded border border-accent-default bg-accent-default px-2 py-1 text-xs text-fg-inverse hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            @click="submit"
          >
            {{ saving ? "…" : t("profiles.actions.save") }}
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>
