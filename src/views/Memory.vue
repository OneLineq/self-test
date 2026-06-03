<script setup lang="ts">
// ============================================================
// 刷题助手 — 练习记忆页面
// ============================================================
import { onMounted, ref, computed, h } from 'vue'
import { useRouter } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  DeleteOutlined,
  ExclamationCircleOutlined,
  BankOutlined,
  RiseOutlined,
  ClockCircleOutlined,
  BookOutlined,
  ThunderboltOutlined,
  BugOutlined,
  FormOutlined,
  ReadOutlined,
  FolderOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import { PracticeModeLabel } from '../types'
import type { PracticeMemoryItem } from '../types'
import { useBankStore } from '../stores/bank'

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

const loading = ref(true)
const stats = ref<GlobalStats | null>(null)
const memoryMap = ref<Record<string, PracticeMemoryItem[]>>({})
const expandedBankId = ref<string | null>(null)
const loadingMemory = ref(false)
const clearing = ref(false)

const typeLabels: Record<string, string> = {
  single: '单选题',
  multiple: '多选题',
  judge: '判断题',
  fill: '填空题',
}

const typeColors: Record<string, string> = {
  single: 'blue',
  multiple: 'purple',
  judge: 'orange',
  fill: 'green',
}

const modeIcons: Record<string, any> = {
  sequential: BookOutlined,
  random: ThunderboltOutlined,
  wrong: BugOutlined,
  exam: FormOutlined,
}

const accuracy = computed(() => {
  if (!stats.value || stats.value.total_practice === 0) return 0
  return Math.round((stats.value.total_correct / stats.value.total_practice) * 100)
})

onMounted(async () => {
  try {
    stats.value = await invoke<GlobalStats>('get_global_stats')
  } catch (e) {
    console.error('加载统计失败', e)
  } finally {
    loading.value = false
  }
  bankStore.fetchBanks()
})

/** 解析正确答案用于显示 */
function formatCorrectAnswer(item: PracticeMemoryItem): string {
  if (!item.correct_answer) return ''
  if (item.correct_answer === '""') return '(空)'
  // correct_answer 是 JSON 编码的：单选如 "\"A\""，多选如 "[\"A\",\"C\"]"
  try {
    const parsed = JSON.parse(item.correct_answer)
    if (Array.isArray(parsed)) return parsed.join(', ')
    return parsed || '(空)'
  } catch {
    return item.correct_answer
  }
}

/** 解析用户答案用于显示 */
function formatUserAnswer(item: PracticeMemoryItem): string {
  if (!item.user_answer) return '(未作答)'
  return item.user_answer
}

/** 展开/收起题库详情 */
async function toggleBank(bankId: string) {
  if (expandedBankId.value === bankId) {
    expandedBankId.value = null
    return
  }
  expandedBankId.value = bankId
  loadingMemory.value = true
  try {
    const records = await invoke<PracticeMemoryItem[]>('get_practice_memory', { bankId })
    memoryMap.value[bankId] = records
  } catch (e) {
    message.error('加载记忆失败')
    console.error(e)
  } finally {
    loadingMemory.value = false
  }
}

/** 清除某题库的记忆 */
async function clearMemory(bankId: string, bankName: string) {
  Modal.confirm({
    title: '清除记忆',
    icon: h(ExclamationCircleOutlined),
    content: `确定要清除题库「${bankName}」的所有刷题记忆吗？\n这将删除所有练习记录并重置题目统计，此操作不可恢复。`,
    okText: '清除',
    okType: 'danger',
    async onOk() {
      clearing.value = true
      try {
        await invoke('clear_practice_memory', { bankId })
        // 刷新数据
        memoryMap.value[bankId] = []
        stats.value = await invoke<GlobalStats>('get_global_stats')
        message.success('已清除记忆')
      } catch (e) {
        message.error('清除失败: ' + e)
      } finally {
        clearing.value = false
      }
    },
  })
}

function getModeColor(mode: string): string {
  switch (mode) {
    case 'sequential': return '#1890ff'
    case 'random': return '#722ed1'
    case 'wrong': return '#f5222d'
    case 'exam': return '#fa8c16'
    default: return '#999'
  }
}
</script>

<template>
  <div class="memory-page">
    <h2 style="margin-bottom: 16px"><ReadOutlined style="margin-right: 6px" />练习记忆</h2>

    <!-- 全局统计 -->
    <a-spin :spinning="loading">
      <a-row :gutter="[16, 16]" style="margin-bottom: 24px">
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small">
            <a-statistic title="累计练习" :value="stats?.total_practice ?? 0" :value-style="{ color: '#1890ff' }">
              <template #prefix><ClockCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small">
            <a-statistic title="正确" :value="stats?.total_correct ?? 0" :value-style="{ color: '#52c41a' }">
              <template #prefix><CheckCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small">
            <a-statistic
              title="错误"
              :value="(stats?.total_practice ?? 0) - (stats?.total_correct ?? 0)"
              :value-style="{ color: '#f5222d' }"
            >
              <template #prefix><CloseCircleOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
        <a-col :xs="12" :sm="12" :md="6">
          <a-card size="small">
            <a-statistic
              title="正确率"
              :value="accuracy"
              suffix="%"
              :precision="0"
              :value-style="{ color: accuracy >= 60 ? '#52c41a' : accuracy >= 40 ? '#fa8c16' : '#f5222d' }"
            >
              <template #prefix><RiseOutlined /></template>
            </a-statistic>
          </a-card>
        </a-col>
      </a-row>
    </a-spin>

    <!-- 各题库记忆 -->
    <h3 style="margin-bottom: 12px"><FolderOutlined style="margin-right: 6px" />各题库记忆详情</h3>

    <a-spin :spinning="bankStore.loading">
      <div v-if="bankStore.banks.length === 0" style="text-align: center; padding: 60px">
        <a-empty description="还没有题库，快去创建吧！">
          <a-button type="primary" @click="router.push('/banks')">创建题库</a-button>
        </a-empty>
      </div>

      <div v-else class="bank-memory-list">
        <a-card
          v-for="bank in bankStore.banks"
          :key="bank.id"
          size="small"
          style="margin-bottom: 12px"
          :class="{ 'bank-expanded': expandedBankId === bank.id }"
        >
          <template #title>
            <div class="bank-header" @click="toggleBank(bank.id)">
              <a-space>
                <BankOutlined />
                <span>{{ bank.name }}</span>
              </a-space>
              <a-space size="large">
                <span style="font-size: 12px; color: #999">
                  共 {{ bank.question_count }} 题 ·
                  已练 {{ stats?.banks?.find(b => b.id === bank.id)?.attempted ?? 0 }} 题
                </span>
                <span
                  v-if="(stats?.banks?.find(b => b.id === bank.id)?.total_practice ?? 0) > 0"
                  :style="{
                    fontSize: '12px',
                    color: (() => {
                      const s = stats?.banks?.find(b => b.id === bank.id)
                      if (!s || s.total_practice === 0) return '#999'
                      const rate = Math.round((s.correct / s.total_practice) * 100)
                      return rate >= 60 ? '#52c41a' : rate >= 40 ? '#fa8c16' : '#f5222d'
                    })(),
                  }"
                >
                  正确率 {{
                    (() => {
                      const s = stats?.banks?.find(b => b.id === bank.id)
                      if (!s || s.total_practice === 0) return '—'
                      return Math.round((s.correct / s.total_practice) * 100) + '%'
                    })()
                  }}
                </span>
                <span v-if="expandedBankId === bank.id" style="color: #1890ff; font-size: 12px">收起</span>
                <span v-else style="color: #999; font-size: 12px">展开</span>
              </a-space>
            </div>
          </template>

          <!-- 展开后的记忆详情 -->
          <div v-if="expandedBankId === bank.id">
            <div v-if="loadingMemory" style="text-align: center; padding: 24px">
              <a-spin />
            </div>

            <div v-else-if="!memoryMap[bank.id] || memoryMap[bank.id].length === 0" style="text-align: center; padding: 24px">
              <a-empty description="暂无刷题记录" />
            </div>

            <div v-else>
              <!-- 题库内统计 -->
              <div class="memory-stats-bar">
                <a-space size="large">
                  <span>共 <b>{{ memoryMap[bank.id].length }}</b> 条记录</span>
                  <span style="color: #52c41a">正确 <b>{{ memoryMap[bank.id].filter(r => r.is_correct).length }}</b></span>
                  <span style="color: #f5222d">错误 <b>{{ memoryMap[bank.id].filter(r => !r.is_correct).length }}</b></span>
                  <span>
                    正确率 <b :style="(() => {
                      const t = memoryMap[bank.id].length
                      const c = memoryMap[bank.id].filter(r => r.is_correct).length
                      const rate = t > 0 ? Math.round((c / t) * 100) : 0
                      return { color: rate >= 60 ? '#52c41a' : rate >= 40 ? '#fa8c16' : '#f5222d' }
                    })()">
                    {{ (() => {
                      const t = memoryMap[bank.id].length
                      const c = memoryMap[bank.id].filter(r => r.is_correct).length
                      return t > 0 ? Math.round((c / t) * 100) + '%' : '—'
                    })() }}
                    </b>
                  </span>
                </a-space>
                <a-button
                  danger
                  size="small"
                  :loading="clearing"
                  @click="clearMemory(bank.id, bank.name)"
                >
                  <DeleteOutlined /> 清除记忆
                </a-button>
              </div>

              <a-divider style="margin: 12px 0" />

              <!-- 记忆记录列表 -->
              <div class="memory-records">
                <div
                  v-for="(record, idx) in memoryMap[bank.id]"
                  :key="record.id"
                  class="memory-record-item"
                >
                  <div class="record-header">
                    <span class="record-index">{{ idx + 1 }}.</span>
                    <a-tag v-if="record.type && typeLabels[record.type]" :color="typeColors[record.type]" size="small">
                      {{ typeLabels[record.type] }}
                    </a-tag>
                    <span class="record-stem">{{ record.stem }}</span>
                    <span class="record-result" :class="record.is_correct ? 'result-correct' : 'result-wrong'">
                      <span v-if="record.is_correct"><CheckCircleOutlined /> 正确</span>
                      <span v-else><CloseCircleOutlined /> 错误</span>
                    </span>
                  </div>
                  <div class="record-detail">
                    <a-space size="middle">
                      <span>
                        <span style="color: #999">你的答案：</span>
                        <span :class="record.is_correct ? 'text-correct' : 'text-wrong'">
                          {{ formatUserAnswer(record) }}
                        </span>
                      </span>
                      <span v-if="!record.is_correct">
                        <span style="color: #999">正确答案：</span>
                        <span class="text-correct">{{ formatCorrectAnswer(record) }}</span>
                      </span>
                      <span>
                        <component :is="modeIcons[record.mode]" :style="{ color: getModeColor(record.mode), fontSize: '12px' }" />
                        <span style="color: #999; margin-left: 2px">{{ PracticeModeLabel[record.mode] || record.mode }}</span>
                      </span>
                      <span style="color: #ccc">|</span>
                      <span style="color: #999; font-size: 12px">{{ record.timestamp }}</span>
                    </a-space>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </a-card>
      </div>
    </a-spin>
  </div>
</template>

<style scoped>
.memory-page {
  max-width: 1000px;
  margin: 0 auto;
}

h3 {
  font-size: 16px;
  font-weight: 600;
}

.bank-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
}

.bank-expanded {
  border-left: 3px solid #1890ff;
}

.memory-stats-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.memory-records {
  max-height: 600px;
  overflow-y: auto;
}

.memory-record-item {
  padding: 10px 12px;
  border: 1px solid #f0f0f0;
  border-radius: 6px;
  margin-bottom: 8px;
  transition: background 0.2s;
}

.memory-record-item:hover {
  background: #fafafa;
}

.record-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.record-index {
  font-weight: bold;
  color: #999;
  font-size: 12px;
}

.record-stem {
  flex: 1;
  font-size: 14px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.record-result {
  font-size: 12px;
  white-space: nowrap;
  flex-shrink: 0;
}

.result-correct {
  color: #52c41a;
}

.result-wrong {
  color: #f5222d;
}

.record-detail {
  font-size: 13px;
  margin-left: 22px;
}

.text-correct {
  color: #52c41a;
  font-weight: 500;
}

.text-wrong {
  color: #f5222d;
  font-weight: 500;
}

:deep(.ant-statistic) {
  text-align: center;
}

:deep(.ant-statistic-title) {
  font-size: 13px;
}
</style>
