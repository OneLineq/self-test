<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — Dashboard 首页（全局统计 + 快速入口）
// ============================================================
import { onMounted, ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/tauri'
import {
  BankOutlined,
  QuestionCircleOutlined,
  ClockCircleOutlined,
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  FormOutlined,
  FilterOutlined,
  RightOutlined,
  RiseOutlined,
  ExclamationCircleOutlined,
  RocketOutlined,
  SearchOutlined,
} from '@ant-design/icons-vue'
import { useBankStore } from '../stores/bank'
import { PracticeModeLabel, bankNameMatches } from '../types'

const router = useRouter()
const bankStore = useBankStore()

interface BankStats {
  id: string
  name: string
  question_count: number
  attempted: number
  correct: number
  total_practice: number
}

interface GlobalStats {
  total_banks: number
  total_questions: number
  total_practice: number
  total_correct: number
  today_practice: number
  wrong_count: number
  banks: BankStats[]
}

const stats = ref<GlobalStats | null>(null)
const loading = ref(true)
const bankKeyword = ref('')

const filteredBanks = computed(() =>
  bankStore.banks.filter(b => bankNameMatches(b.name, bankKeyword.value)),
)

const bankStatsById = computed(() => {
  const map: Record<string, BankStats> = {}
  for (const b of stats.value?.banks ?? []) map[b.id] = b
  return map
})

const accuracy = computed(() => {
  if (!stats.value || stats.value.total_practice === 0) return 0
  return Math.round((stats.value.total_correct / stats.value.total_practice) * 100)
})

const modeCards = [
  { key: 'sequential', icon: BookOutlined, color: '#1890ff', bg: '#e6f7ff' },
  { key: 'random', icon: ThunderboltOutlined, color: '#722ed1', bg: '#f9f0ff' },
  { key: 'wrong', icon: BugOutlined, color: '#f5222d', bg: '#fff2f0' },
  { key: 'topic', icon: FilterOutlined, color: '#13c2c2', bg: '#e6fffb' },
  { key: 'exam', icon: FormOutlined, color: '#fa8c16', bg: '#fff7e6' },
]

onMounted(() => {
  invoke<GlobalStats>('get_global_stats')
    .then(v => { stats.value = v })
    .catch(e => console.error('加载统计失败', e))
    .finally(() => { loading.value = false })
  bankStore.fetchBanks()
})

function goMode(mode: string) {
  router.push(`/${mode}`)
}

function goBankPractice(bankId: string, mode: string) {
  if (mode === 'exam') {
    router.push(`/exam/${bankId}`)
  } else if (mode === 'topic') {
    router.push(`/topic/${bankId}`)
  } else {
    router.push(`/practice/${bankId}?mode=${mode}`)
  }
}

function modeLabel(key: string): string {
  if (key === 'topic') return '主题练习'
  return PracticeModeLabel[key as keyof typeof PracticeModeLabel] || key
}

function goBankManage(bankId: string) {
  router.push(`/questions/${bankId}`)
}
</script>

<template>
  <div class="dashboard">
    <!-- 欢迎区 -->
    <a-alert
      message="欢迎使用理论训练考核系统"
      description="选择练习模式开始刷题，或从下方题库快速进入。支持顺序、随机、错题、主题筛选与模拟考试。"
      type="info"
      show-icon
      closable
      style="margin-bottom: 24px"
    />

    <!-- 全局统计卡片 -->
    <a-spin :spinning="loading">
      <a-row :gutter="[16, 16]" style="margin-bottom: 24px">
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="题库总数"
              :value="stats?.total_banks ?? 0"
              :value-style="{ color: '#1890ff' }"
            >
              <template #prefix><BankOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="题目总数"
              :value="stats?.total_questions ?? 0"
              :value-style="{ color: '#722ed1' }"
            >
              <template #prefix><QuestionCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="累计练习"
              :value="stats?.total_practice ?? 0"
              :value-style="{ color: '#fa8c16' }"
            >
              <template #prefix><ClockCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="正确率"
              :value="accuracy"
              suffix="%"
              :precision="0"
              :value-style="{
                color: accuracy >= 60 ? '#52c41a' : accuracy >= 40 ? '#fa8c16' : '#f5222d',
              }"
            >
              <template #prefix><RiseOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
      </a-row>

      <!-- 今日练习 + 错题数 -->
      <a-row :gutter="[16, 16]" style="margin-bottom: 24px">
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="今日练习"
              :value="stats?.today_practice ?? 0"
              :value-style="{ color: '#13c2c2' }"
            >
              <template #prefix><RiseOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small" hoverable>
            <a-statistic
              title="待复习错题"
              :value="stats?.wrong_count ?? 0"
              :value-style="{
                color: (stats?.wrong_count ?? 0) > 0 ? '#f5222d' : '#52c41a',
              }"
            >
              <template #prefix><ExclamationCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
      </a-row>
    </a-spin>

    <!-- 练习模式快捷入口 -->
    <h3 style="margin-bottom: 12px">
      <RocketOutlined style="margin-right: 6px" />快速开始练习
    </h3>
    <a-row :gutter="[16, 16]" style="margin-bottom: 24px">
      <a-col
        v-for="m in modeCards"
        :key="m.key"
        :xs="12"
        :sm="12"
        :md="8"
        :lg="8"
        :xl="4"
      >
        <a-card
          hoverable
          size="small"
          :style="{ borderLeft: `4px solid ${m.color}`, background: m.bg }"
          @click="goMode(m.key)"
        >
          <a-space>
            <component :is="m.icon" :style="{ fontSize: '24px', color: m.color }" />
            <div>
              <div style="font-weight: bold; font-size: 15px">
                {{ modeLabel(m.key) }}
              </div>
              <div style="font-size: 12px; color: #999; margin-top: 2px">
                <template v-if="m.key === 'sequential'">按题库顺序逐题练习</template>
                <template v-else-if="m.key === 'random'">随机打乱题目顺序</template>
                <template v-else-if="m.key === 'wrong'">只练习做错的题目</template>
                <template v-else-if="m.key === 'topic'">关键词筛选后练习</template>
                <template v-else-if="m.key === 'exam'">限时模拟考试模式</template>
              </div>
            </div>
          </a-space>
        </a-card>
      </a-col>
    </a-row>

    <!-- 题库列表（带统计） -->
    <div style="margin-bottom: 12px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap">
      <h3 style="margin: 0">📚 题库概览</h3>
      <a-input
        v-if="bankStore.banks.length > 0"
        v-model:value="bankKeyword"
        placeholder="搜索题库名称"
        allow-clear
        style="width: 240px; margin-left: auto"
      >
        <template #prefix><SearchOutlined style="color: #bfbfbf" /></template>
      </a-input>
    </div>
    <a-spin :spinning="bankStore.loading">
      <div v-if="bankStore.banks.length === 0" style="text-align: center; padding: 40px">
        <a-empty description="还没有题库，快去创建吧！">
          <a-button type="primary" @click="router.push('/banks')">
            创建题库
          </a-button>
        </a-empty>
      </div>

      <div v-else-if="filteredBanks.length === 0" style="text-align: center; padding: 40px">
        <a-empty description="未找到匹配题库" />
      </div>

      <a-row :gutter="[16, 16]" v-else>
        <a-col
          v-for="bank in filteredBanks"
          :key="bank.id"
          :xs="24"
          :sm="24"
          :md="12"
          :lg="8"
        >
          <a-card :title="bank.name" size="small" hoverable>
            <template #extra>
              <a-button type="link" size="small" @click="goBankManage(bank.id)">
                管理 <RightOutlined />
              </a-button>
            </template>

            <!-- 题库统计 -->
            <a-descriptions :column="2" size="small" style="margin-bottom: 8px">
              <a-descriptions-item label="题目数">
                {{ bank.question_count }}
              </a-descriptions-item>
              <a-descriptions-item label="已练习">
                {{ bankStatsById[bank.id]?.attempted ?? 0 }} 题
              </a-descriptions-item>
              <a-descriptions-item label="正确率">
                <span v-if="(bankStatsById[bank.id]?.total_practice ?? 0) > 0"
                  :style="{ color: '#52c41a' }">
                  {{ Math.round(((bankStatsById[bank.id]?.correct ?? 0) / (bankStatsById[bank.id]?.total_practice ?? 1)) * 100) }}%
                </span>
                <span v-else style="color: #999">—</span>
              </a-descriptions-item>
              <a-descriptions-item label="创建时间">
                {{ bank.created_at }}
              </a-descriptions-item>
            </a-descriptions>

            <!-- 练习模式按钮 -->
            <a-space wrap :size="[6, 6]">
              <a-button
                v-for="m in modeCards"
                :key="m.key"
                size="small"
                :style="{ borderColor: m.color, color: m.color }"
                :disabled="bank.question_count === 0 || (m.key === 'wrong' && bank.wrong_count === 0)"
                @click="goBankPractice(bank.id, m.key)"
              >
                <component :is="m.icon" />
                {{ modeLabel(m.key) }}
              </a-button>
            </a-space>
          </a-card>
        </a-col>
      </a-row>
    </a-spin>
  </div>
</template>

<style scoped>
.dashboard {
  max-width: 1200px;
  margin: 0 auto;
}

h3 {
  font-size: 16px;
  font-weight: 600;
}

/* 让统计卡片内容居中 */
:deep(.ant-statistic) {
  text-align: center;
}

:deep(.ant-statistic-title) {
  font-size: 13px;
}
</style>
