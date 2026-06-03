<script setup lang="ts">
// ============================================================
// 刷题助手 — 模拟考试（含考前设置）
// ============================================================
import { onMounted, onBeforeUnmount, ref, computed, h } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  ExclamationCircleOutlined,
  ClockCircleOutlined,
} from '@ant-design/icons-vue'
import { usePracticeStore } from '../stores/practice'
import { invoke } from '@tauri-apps/api/tauri'
import { isChoiceType } from '../types'
import type { Question } from '../types'

const route = useRoute()
const router = useRouter()
const store = usePracticeStore()

const bankId = route.params.bankId as string

// ===== 设置状态 =====
const loading = ref(true)
const allQuestions = ref<Question[]>([])
const settingStep = ref<'settings' | 'exam'>('settings')

// 题库中现有的题型
const availableTypes = ref<string[]>([])
// 各题型对应的题目总数
const typeTotalCounts = ref<Record<string, number>>({})
// 各题型要抽取的题目数
const perTypeLimits = ref<Record<string, number>>({})
/** 不定项模式：选择题不区分单选/多选，可自由选择一项或多项 */
const indeterminateMode = ref(false)
// 考试时间（分钟）
const examMinutes = ref(30)

// ===== 考试状态 =====
const remainingSeconds = ref(0)
const examSubmitted = ref(false)
const examScore = ref({ correct: 0, total: 0, percentage: 0 })
let timerInterval: number | null = null

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

onMounted(async () => {
  try {
    // 先拉取全部试题，用于展示统计
    allQuestions.value = await invoke<Question[]>('get_practice_questions', {
      bankId,
      mode: 'sequential',
      questionTypes: null,
      limit: null,
    })
    // 提取题型并按题型统计数量
    const counts: Record<string, number> = {}
    for (const q of allQuestions.value) {
      if (q.type) {
        counts[q.type] = (counts[q.type] || 0) + 1
      }
    }
    availableTypes.value = Object.keys(counts).sort()
    typeTotalCounts.value = { ...counts }
    // 默认每种题型全部抽取
    perTypeLimits.value = { ...counts }
    // 默认时间：每 5 题加 1 分钟，最少 5 分钟
    const totalQ = allQuestions.value.length
    examMinutes.value = Math.max(5, Math.ceil(totalQ / 5) * 1.5)
    loading.value = false
  } catch (e) {
    message.error('加载题目失败')
    router.back()
  }
})

onBeforeUnmount(() => {
  if (timerInterval) clearInterval(timerInterval)
})

// 离页确认：考试进行中时拦截侧边栏误触
// 如果侧边栏已弹过确认框（skipLeaveConfirm），则直接放行
onBeforeRouteLeave((_to, _from, next) => {
  if (store.skipLeaveConfirm) {
    store.skipLeaveConfirm = false
    next()
    return
  }
  if (settingStep.value === 'exam' && !examSubmitted.value) {
    Modal.confirm({
      title: '退出考试？',
      icon: h(ExclamationCircleOutlined),
      content: `考试正在进行中（${store.answeredCount}/${store.totalCount} 题已答），退出后本次作答记录不会丢失。`,
      okText: '退出',
      cancelText: '继续考试',
      onOk: () => next(),
      onCancel: () => next(false),
    })
  } else {
    next()
  }
})

// ===== 计算属性 =====

/** 总共要抽取的题目数（各题型之和） */
const totalSelectedCount = computed(() => {
  return Object.values(perTypeLimits.value).reduce((sum, v) => sum + v, 0)
})

/** 题库总题数 */
const totalQuestionsCount = computed(() => allQuestions.value.length)

const timeDisplay = computed(() => {
  const m = examMinutes.value
  if (m >= 60) {
    return `${Math.floor(m / 60)} 小时 ${m % 60 > 0 ? `${m % 60} 分钟` : ''}`
  }
  return `${m} 分钟`
})

const timePerQuestion = computed(() => {
  const count = totalSelectedCount.value
  if (count === 0) return 0
  return Math.round((examMinutes.value * 60) / count)
})

// ===== 开始考试 =====

async function startExam() {
  settingStep.value = 'exam'
  examSubmitted.value = false

  // 筛选出抽取数量 > 0 的题型
  const ptl: Record<string, number> = {}
  for (const [type, count] of Object.entries(perTypeLimits.value)) {
    if (count > 0) ptl[type] = count
  }

  // 如果有题型没启用（count === 0），只传启用的题型
  const activeTypes = Object.keys(ptl)

  await store.loadQuestions(
    bankId,
    'exam',
    activeTypes.length > 0 ? activeTypes : undefined,
    undefined,
    ptl,
  )
  remainingSeconds.value = examMinutes.value * 60
  startTimer()
}

function startTimer() {
  timerInterval = window.setInterval(() => {
    remainingSeconds.value--
    if (remainingSeconds.value <= 0) {
      submitExam(true)
    }
  }, 1000)
}

// ===== 答题 =====

function selectOption(questionId: string, optIndex: number) {
  if (examSubmitted.value) return
  const letter = String.fromCharCode(65 + optIndex)
  const q = store.questions.find(q => q.id === questionId)
  if (!q) return
  const isChoice = isChoiceType(q.type)
  // 不定项模式下的选择题 或 原有多选题：切换选中
  const isMulti = (indeterminateMode.value && isChoice) || Array.isArray(q.answer)
  if (isMulti) {
    let current = store.userAnswers.get(questionId) || ''
    const parts = current ? current.split(',').filter(Boolean) : []
    if (parts.includes(letter)) {
      store.userAnswers.set(questionId, parts.filter(p => p !== letter).join(','))
    } else {
      parts.push(letter)
      store.userAnswers.set(questionId, parts.join(','))
    }
  } else {
    // 单选题：替换
    store.userAnswers.set(questionId, letter)
  }
}

function getOptionClass(questionId: string, optIndex: number): string {
  const letter = String.fromCharCode(65 + optIndex)
  const selected = store.userAnswers.get(questionId)
  if (!selected) return ''
  // 多选题：检查该字母是否在已选列表中
  const parts = selected.split(',').filter(Boolean)
  if (parts.includes(letter)) return 'option-selected'
  return ''
}

function submitExam(auto = false) {
  if (examSubmitted.value) return
  const unanswered = store.totalCount - store.answeredCount

  if (auto) {
    // 自动交卷（倒计时归零）
    doSubmit()
    message.warning('时间到！已自动交卷')
    return
  }

  Modal.confirm({
    title: '交卷确认',
    icon: h(ExclamationCircleOutlined),
    content: `已作答 ${store.answeredCount} / ${store.totalCount} 题${unanswered > 0 ? `，还有 ${unanswered} 题未答` : ''}，确定交卷吗？`,
    okText: '交卷',
    cancelText: '继续答题',
    async onOk() {
      doSubmit()
    },
  })
}

async function doSubmit() {
  if (timerInterval) clearInterval(timerInterval)
  examSubmitted.value = true
  await store.submitExamAnswers()
  examScore.value = {
    correct: store.correctCount,
    total: store.totalCount,
    percentage: store.totalCount > 0
      ? Math.round((store.correctCount / store.totalCount) * 100)
      : 0,
  }
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}
</script>

<template>
  <div class="exam-page">
    <h2 style="margin-bottom: 16px">📋 模拟考试</h2>

    <!-- ===== 加载中 ===== -->
    <div v-if="loading" style="text-align: center; padding: 80px">
      <a-spin size="large" />
    </div>

    <!-- ===== 考试设置 ===== -->
    <div v-else-if="settingStep === 'settings'" class="settings-container">
      <a-card title="考试设置" style="max-width: 600px; margin: 0 auto">
        <a-form layout="vertical">
          <!-- 按题型抽取 -->
          <a-form-item
            label="各题型抽取数量"
            v-if="availableTypes.length > 0"
          >
            <template #extra>
              题库共 {{ totalQuestionsCount }} 题，当前共抽取 {{ totalSelectedCount }} 题
            </template>
            <div
              v-for="t in availableTypes"
              :key="t"
              style="display: flex; align-items: center; gap: 12px; margin-bottom: 10px"
            >
              <a-tag :color="typeColors[t] || 'default'" style="min-width: 60px; text-align: center">
                {{ typeLabels[t] || t }}
              </a-tag>
              <a-slider
                v-model:value="perTypeLimits[t]"
                :min="0"
                :max="typeTotalCounts[t]"
                style="flex: 1; margin: 0"
              />
              <a-input-number
                v-model:value="perTypeLimits[t]"
                :min="0"
                :max="typeTotalCounts[t]"
                style="width: 72px"
              />
              <span style="color: #999; font-size: 12px; white-space: nowrap">
                / {{ typeTotalCounts[t] }} 题
              </span>
            </div>
          </a-form-item>

          <!-- 没有题型时显示总题数 -->
          <a-form-item label="题目数量" v-if="availableTypes.length === 0">
            <span>共 {{ totalQuestionsCount }} 题</span>
          </a-form-item>

          <!-- 不定项模式 -->
          <a-form-item>
            <template #label>
              <span>
                不定项选择
                <a-tooltip title="开启后，选择题不显示单选/多选标签，您可以自由选择一项或多项后提交">
                  <ExclamationCircleOutlined style="color: #999; font-size: 12px" />
                </a-tooltip>
              </span>
            </template>
            <a-switch v-model:checked="indeterminateMode" />
            <span style="margin-left: 8px; color: #999; font-size: 12px">
              不区分单选/多选，可自由选择一项或多项
            </span>
          </a-form-item>

          <!-- 考试时间 -->
          <a-form-item label="考试时间">
            <a-row :gutter="16">
              <a-col :span="18">
                <a-slider
                  v-model:value="examMinutes"
                  :min="5"
                  :max="120"
                  :marks="{ 5: '5分', 30: '30分', 60: '1小时', 120: '2小时' }"
                />
              </a-col>
              <a-col :span="6" style="text-align: right">
                <span style="font-size: 18px; font-weight: bold; color: #1890ff">
                  {{ examMinutes }}
                </span>
                <span style="color: #999"> 分钟</span>
              </a-col>
            </a-row>
            <div style="margin-top: 8px; color: #999; font-size: 12px">
              <clock-circle-outlined />
              {{ timeDisplay }}，平均每题 {{ timePerQuestion }} 秒
            </div>
          </a-form-item>

          <!-- 开始按钮 -->
          <a-form-item style="text-align: center">
            <a-button
              type="primary"
              size="large"
              :disabled="totalSelectedCount === 0"
              @click="startExam"
            >
              开始考试（{{ totalSelectedCount }} 题，{{ timeDisplay }}）
            </a-button>
          </a-form-item>
        </a-form>
      </a-card>
    </div>

    <!-- ===== 考试中 ===== -->
    <div v-else-if="settingStep === 'exam' && !examSubmitted">
      <!-- 顶部信息栏 -->
      <div class="exam-header">
        <a-tag color="orange">模拟考试</a-tag>
        <span
          style="flex: 1; text-align: center; font-size: 20px; font-weight: bold; font-family: monospace"
          :style="{ color: remainingSeconds < 60 ? 'red' : remainingSeconds < 180 ? '#fa8c16' : '#333' }"
        >
          ⏱ {{ formatTime(remainingSeconds) }}
        </span>
        <span>{{ store.answeredCount }} / {{ store.totalCount }} 已答</span>
        <a-button
          type="primary"
          danger
          style="margin-left: 16px"
          :disabled="examSubmitted"
          @click="submitExam(false)"
        >
          交卷
        </a-button>
      </div>

      <a-divider />

      <!-- 题目列表 -->
      <div class="question-list">
        <div
          v-for="(q, qIdx) in store.questions"
          :key="q.id"
          class="exam-question"
        >
          <div class="question-stem">
            <span class="q-number">{{ qIdx + 1 }}.</span>
            <a-tag v-if="indeterminateMode && isChoiceType(q.type)" color="orange" size="small" style="margin-right: 6px">
              不定项选择
            </a-tag>
            <a-tag v-else-if="q.type" :color="typeColors[q.type]" size="small" style="margin-right: 6px">
              {{ typeLabels[q.type] || q.type }}
            </a-tag>
            {{ q.stem }}
          </div>

          <div class="options-list">
            <div
              v-for="(opt, idx) in q.options"
              :key="idx"
              class="option-item"
              :class="getOptionClass(q.id, idx)"
              @click="selectOption(q.id, idx)"
            >
              <span class="option-letter">{{ String.fromCharCode(65 + idx) }}</span>
              <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
            </div>
          </div>

          <a-divider />
        </div>
      </div>
    </div>

    <!-- ===== 考试结果 ===== -->
    <a-result
      v-else
      :status="examScore.percentage >= 60 ? 'success' : 'error'"
      :title="examScore.percentage >= 60 ? '考试通过！' : '还需努力'"
      :sub-title="`得分：${examScore.correct} / ${examScore.total}（${examScore.percentage}%）`"
    >
      <template #extra>
        <a-space>
          <a-button @click="router.push('/')">返回首页</a-button>
          <a-button type="primary" @click="router.go(0)">重新考试</a-button>
        </a-space>
      </template>
    </a-result>
  </div>
</template>

<style scoped>
.exam-page {
  max-width: 900px;
  margin: 0 auto;
}

.settings-container {
  margin-top: 24px;
}

.exam-header {
  display: flex;
  align-items: center;
}

.question-stem {
  font-size: 15px;
  line-height: 1.6;
  margin-bottom: 8px;
}

.q-number {
  font-weight: bold;
  color: #fa8c16;
  margin-right: 6px;
}

.options-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-left: 28px;
}

.option-item {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  border: 1px solid #d9d9d9;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.option-item:hover {
  border-color: #fa8c16;
  background: #fff7e6;
}

.option-selected {
  border-color: #fa8c16 !important;
  background: #fff7e6 !important;
}

.option-selected .option-letter {
  background: #fa8c16;
  color: #fff;
}

.option-letter {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: #f0f0f0;
  font-weight: bold;
  margin-right: 10px;
  flex-shrink: 0;
  font-size: 13px;
}

.option-text {
  flex: 1;
  font-size: 14px;
}
</style>
