<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import {
  ArrowLeft,
  BadgeInfo,
  CalendarDays,
  Landmark,
  UserRound,
  WalletCards,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import { formatCurrency, useKoperasiStore } from "../store/koperasi";

const route = useRoute();
const { members, loans, transactions } = useKoperasiStore();

const member = computed(() =>
  members.find((item) => item.id === String(route.params.id)),
);
const memberLoans = computed(() =>
  member.value
    ? loans.filter((loan) => loan.memberId === member.value?.id)
    : [],
);
const memberTransactions = computed(() => {
  if (!member.value) return [];
  const memberName = member.value.name.toLocaleLowerCase("id-ID");
  return transactions
    .filter((item) => item.memberName.toLocaleLowerCase("id-ID") === memberName)
    .slice(0, 5);
});
</script>

<template>
  <div class="page-stack">
    <PageHeader :title="member?.name ?? 'Detail anggota'">
      <template #actions>
        <RouterLink class="button button--secondary" to="/members">
          <ArrowLeft :size="18" /> Kembali ke anggota
        </RouterLink>
      </template>
    </PageHeader>

    <section v-if="member" class="member-detail member-detail--page panel">
      <section class="member-detail__identity">
        <span class="member-detail__avatar"><UserRound :size="28" /></span>
        <div>
          <small>{{ member.memberNumber }} · {{ member.id }}</small>
          <h3>{{ member.name }}</h3>
          <div class="member-detail__meta">
            <span><CalendarDays :size="14" /> {{ member.joinedAt }}</span>
            <StatusPill
              :label="member.status"
              :tone="member.status === 'Aktif' ? 'success' : 'neutral'"
            />
          </div>
        </div>
      </section>

      <section class="member-detail__metrics">
        <article>
          <span>Total simpanan</span>
          <strong>{{ formatCurrency(member.savings) }}</strong>
        </article>
        <article>
          <span>Saldo pinjaman</span>
          <strong>{{ formatCurrency(member.loanBalance) }}</strong>
        </article>
        <article>
          <span>Kontrak pinjaman</span>
          <strong>{{ memberLoans.length }}</strong>
        </article>
      </section>

      <section class="member-detail__section">
        <div class="member-detail__section-title">
          <WalletCards :size="17" />
          <h3>Rincian simpanan</h3>
        </div>
        <div class="member-detail__breakdown">
          <div>
            <span>Simpanan pokok</span>
            <strong>{{ formatCurrency(member.principalSavings) }}</strong>
          </div>
          <div>
            <span>Simpanan wajib</span>
            <strong>{{ formatCurrency(member.mandatorySavings) }}</strong>
          </div>
          <div>
            <span>Simpanan manasuka</span>
            <strong>{{ formatCurrency(member.voluntarySavings) }}</strong>
          </div>
        </div>
      </section>

      <section class="member-detail__section">
        <div class="member-detail__section-title">
          <Landmark :size="17" />
          <h3>Pinjaman</h3>
        </div>
        <div v-if="memberLoans.length" class="member-detail__list">
          <div v-for="loan in memberLoans" :key="loan.id">
            <div>
              <strong>{{ loan.id }}</strong>
              <small>{{ loan.interestType }} · {{ loan.tenor }} bulan</small>
            </div>
            <div class="member-detail__list-value">
              <strong>{{ formatCurrency(loan.balance) }}</strong>
              <StatusPill
                :label="loan.status"
                :tone="
                  loan.status === 'Berjalan'
                    ? 'success'
                    : loan.status === 'Perlu review'
                      ? 'warning'
                      : loan.status === 'Draf'
                        ? 'info'
                        : 'neutral'
                "
              />
            </div>
          </div>
        </div>
        <p v-else class="empty-inline">Belum ada pinjaman untuk anggota ini.</p>
      </section>

      <section class="member-detail__section">
        <div class="member-detail__section-title">
          <BadgeInfo :size="17" />
          <h3>Aktivitas terbaru</h3>
        </div>
        <div v-if="memberTransactions.length" class="member-detail__list">
          <div v-for="item in memberTransactions" :key="item.id">
            <div>
              <strong>{{ item.description }}</strong>
              <small>{{ item.date }} · {{ item.reference }}</small>
            </div>
            <strong
              :class="item.direction === 'Masuk' ? 'amount-in' : 'amount-out'"
            >
              {{ item.direction === "Masuk" ? "+" : "−"
              }}{{ formatCurrency(item.amount) }}
            </strong>
          </div>
        </div>
        <p v-else class="empty-inline">Belum ada aktivitas transaksi.</p>
      </section>
    </section>

    <section v-else class="panel member-detail__not-found">
      <UserRound :size="32" />
      <strong>Anggota tidak ditemukan</strong>
      <p>ID anggota pada alamat ini tidak tersedia.</p>
      <RouterLink class="button button--primary" to="/members">
        Kembali ke daftar anggota
      </RouterLink>
    </section>
  </div>
</template>
