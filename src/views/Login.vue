<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { KeyRound, User } from "lucide-vue-next";

const router = useRouter();
const username = ref("");
const password = ref("");
const isError = ref(false);

function onLogin() {
  if (username.value && password.value) {
    // For mock, any non-empty credential works
    router.push("/loans");
  } else {
    isError.value = true;
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-shell">
      <div class="login-hero">
        <p class="login-hero__eyebrow">Koperasi digital</p>
        <h1 class="login-hero__title">KOPKAR</h1>
        <h2 class="login-hero__subtitle">Masuk ke Sistem Koperasi</h2>
      </div>

      <div class="login-card">
        <form class="login-form" @submit.prevent="onLogin">
          <div v-if="isError" class="login-alert" role="alert">
            Pastikan nama pengguna dan kata sandi telah diisi dengan benar.
          </div>

          <div class="login-field">
            <label for="username" class="login-field__label"
              >Nama Pengguna</label
            >
            <div class="login-field__control">
              <div class="login-field__icon" aria-hidden="true">
                <User class="login-field__icon-svg" />
              </div>
              <input
                id="username"
                v-model="username"
                type="text"
                class="login-field__input"
                placeholder="Masukkan admin"
              />
            </div>
          </div>

          <div class="login-field">
            <label for="password" class="login-field__label">Kata Sandi</label>
            <div class="login-field__control">
              <div class="login-field__icon" aria-hidden="true">
                <KeyRound class="login-field__icon-svg" />
              </div>
              <input
                id="password"
                v-model="password"
                type="password"
                class="login-field__input"
                placeholder="Masukkan kata sandi"
              />
            </div>
          </div>

          <div class="login-form__actions">
            <button type="submit" class="login-form__submit">
              Masuk Sistem
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
}

.login-shell {
  width: min(100%, 620px);
}

.login-hero {
  text-align: center;
}

.login-hero__eyebrow {
  margin: 0;
  font-size: 12px;
  font-weight: 800;
  letter-spacing: 0.32em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.login-hero__title {
  margin: 16px 0 0;
  font-size: clamp(3rem, 7vw, 4.5rem);
  font-weight: 800;
  letter-spacing: -0.08em;
  color: var(--color-primary-strong);
}

.login-hero__subtitle {
  margin: 16px 0 0;
  font-size: clamp(1.4rem, 3vw, 2rem);
  font-weight: 700;
  color: var(--color-text);
}

.login-card {
  margin-top: 36px;
  padding: 40px;
  border: 1px solid rgba(255, 255, 255, 0.72);
  border-radius: 32px;
  background: rgba(255, 255, 255, 0.86);
  box-shadow: var(--shadow-card);
  backdrop-filter: blur(18px);
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 28px;
}

.login-alert {
  padding: 18px 20px;
  border: 1px solid rgba(185, 28, 28, 0.14);
  border-radius: 22px;
  background: rgba(254, 242, 242, 0.92);
  color: var(--color-danger);
  font-size: 1rem;
  font-weight: 700;
  line-height: 1.6;
}

.login-field__label {
  display: block;
  margin-bottom: 12px;
  font-size: 1rem;
  font-weight: 700;
  color: var(--color-text);
}

.login-field__control {
  position: relative;
}

.login-field__icon {
  position: absolute;
  top: 50%;
  left: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  color: var(--color-text-soft);
  transform: translateY(-50%);
  pointer-events: none;
}

.login-field__icon-svg {
  width: 24px;
  height: 24px;
}

.login-field__input {
  width: 100%;
  padding: 18px 20px 18px 60px;
  border: 1px solid var(--color-border);
  border-radius: 22px;
  background: var(--color-surface-muted);
  color: var(--color-text);
  font-size: 1rem;
  transition:
    border-color 160ms ease,
    box-shadow 160ms ease,
    background-color 160ms ease;
}

.login-field__input::placeholder {
  color: var(--color-text-soft);
}

.login-field__input:hover {
  border-color: var(--color-border-strong);
}

.login-field__input:focus-visible {
  border-color: var(--color-primary);
  outline: none;
  background: var(--color-surface-strong);
  box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.14);
}

.login-form__actions {
  padding-top: 8px;
}

.login-form__submit {
  width: 100%;
  padding: 18px 24px;
  border-radius: 24px;
  background: linear-gradient(135deg, var(--color-primary), #2563eb);
  box-shadow: 0 18px 36px rgba(29, 78, 216, 0.22);
  color: #ffffff;
  cursor: pointer;
  font-size: 1.1rem;
  font-weight: 800;
  letter-spacing: 0.01em;
  transition:
    transform 160ms ease,
    box-shadow 160ms ease,
    filter 160ms ease;
}

.login-form__submit:hover,
.login-form__submit:focus-visible {
  transform: translateY(-1px);
  box-shadow: 0 22px 40px rgba(29, 78, 216, 0.28);
  filter: saturate(1.05);
  outline: none;
}

@media (max-width: 640px) {
  .login-page {
    padding: 24px 16px;
  }

  .login-card {
    margin-top: 28px;
    padding: 28px 20px;
    border-radius: 24px;
  }
}
</style>
