<script setup lang="ts">
// ============================================================
// 刷题助手 — 模拟考试（含考前设置）
// ============================================================
import { onMounted, onBeforeUnmount, ref, computed, watch, h } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  ExclamationCircleOutlined,
  ClockCircleOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  CheckOutlined,
  CloseOutlined,
  DownOutlined,
  LeftOutlined,
  RightOutlined,
} from '@ant-design/icons-vue'
import { usePracticeStore } from '../stores/practice'
import { invoke } from '@tauri-apps/api/tauri'
import { isChoiceType, isMultiAnswer, normalizeQuestion } from '../types'
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
/** 打乱选项顺序模式 */
const shuffleMode = ref(false)
/** 各题目的选项排列映射：questionId → displayIdx → originalIdx */
const optionShuffleMaps = ref<Map<string, number[]>>(new Map())

/** 考试结果回顾分页 */
const reviewPage = ref(1)
const reviewPageSize = 20
const paginatedReviewQuestions = computed(() => {
  const start = (reviewPage.value - 1) * reviewPageSize
  return store.questions.slice(start, start + reviewPageSize)
})

/** 生成随机排列（Fisher-Yates） */
function generateShuffle(n: number): number[] {
  const indices = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [indices[i], indices[j]] = [indices[j], indices[i]]
  }
  return indices
}

/** 重新生成所有题目的选项排列 */
function regenerateAllShuffles() {
  if (!shuffleMode.value) {
    optionShuffleMaps.value = new Map()
    return
  }
  const map = new Map<string, number[]>()
  for (const q of store.questions) {
    if (q.options.length > 0) {
      map.set(q.id, generateShuffle(q.options.length))
    }
  }
  optionShuffleMaps.value = map
}

/** 打乱模式下，获取题目某显示位置对应的原始选项字母 */
function getMockOriginalLetter(qId: string, displayIdx: number): string {
  const map = optionShuffleMaps.value.get(qId)
  const originalIdx = shuffleMode.value && map ? map[displayIdx] : displayIdx
  return String.fromCharCode(65 + originalIdx)
}

// 监听打乱开关，重新生成排列
watch(shuffleMode, () => {
  regenerateAllShuffles()
})

// 考试时间（分钟）
const examMinutes = ref(30)

// ===== 考试状态 =====
const remainingSeconds = ref(0)
const examSubmitted = ref(false)
const examScore = ref({ correct: 0, total: 0, percentage: 0 })
let timerInterval: number | null = null

/** 未设置题型的题目归入此分类 */
const UNTYPED = '__untagged__'

const typeLabels: Record<string, string> = {
  single: '单选题',
  multiple: '多选题',
  judge: '判断题',
  fill: '填空题',
  [UNTYPED]: '未分类',
}

const typeColors: Record<string, string> = {
  single: 'blue',
  multiple: 'purple',
  judge: 'orange',
  fill: 'green',
  [UNTYPED]: 'default',
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  try {
    // 与试题管理使用同一接口，确保题型字段一致
    const rawQuestions = await invoke<Question[]>('list_questions', { bankId })
    allQuestions.value = rawQuestions.map(normalizeQuestion)
    // 提取题型并按题型统计数量（normalize 后直接使用 type）
    const counts: Record<string, number> = {}
    for (const q of allQuestions.value) {
      const t = q.type || UNTYPED
      counts[t] = (counts[t] || 0) + 1
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
  window.removeEventListener('keydown', handleKeydown)
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

/** 滑块上限随手动输入扩展，避免超出 120 分钟后滑块顶死 */
const examMinutesSliderMax = computed(() => Math.max(120, examMinutes.value))

// ===== 开始考试 =====

async function startExam() {
  if (totalSelectedCount.value === 0 && allQuestions.value.length === 0) {
    message.warning('题库暂无题目')
    return
  }

  settingStep.value = 'exam'
  examSubmitted.value = false
  reviewPage.value = 1

  // 筛选出抽取数量 > 0 的题型（未分类题型传空字符串给后端）
  const ptl: Record<string, number> = {}
  for (const [type, count] of Object.entries(perTypeLimits.value)) {
    if (count > 0) {
      ptl[type === UNTYPED ? '' : type] = count
    }
  }

  if (Object.keys(ptl).length > 0) {
    await store.loadQuestions(bankId, 'exam', undefined, undefined, ptl)
  } else {
    // 兜底：全部题目参与考试
    await store.loadQuestions(bankId, 'exam')
  }

  if (store.totalCount === 0) {
    settingStep.value = 'settings'
    message.warning('未能抽取到题目，请检查题型设置或确认题目已设置题型')
    return
  }

  // 不定项模式：打乱题目顺序，避免单选/多选按题型块状排列
  if (indeterminateMode.value) {
    store.shuffleQuestionOrder()
  }

  regenerateAllShuffles()
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
  const letter = getMockOriginalLetter(questionId, optIndex)
  const q = store.questions.find(q => q.id === questionId)
  if (!q) return
  const isChoice = isChoiceType(q.type)
  // 不定项模式下的选择题 或 原有多选题：切换选中
  const isMulti = (indeterminateMode.value && isChoice) || isMultiAnswer(q.answer)
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
  const letter = getMockOriginalLetter(questionId, optIndex)
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

/** 将答案字母转为选项文字，如 "A" → "A. 选项内容" */
function formatAnswerText(q: Question, answer: string): string {
  if (!answer) return '(未作答)'
  const parts = answer.split(',').filter(Boolean)
  return parts.map(p => {
    const idx = p.charCodeAt(0) - 65
    if (idx >= 0 && idx < q.options.length) {
      return `${p}. ${q.options[idx].replace(/^[A-D][.、]\s*/, '')}`
    }
    return p
  }).join('；')
}

/** 用户该题是否答错 */
function isWrong(q: Question): boolean {
  return store.userAnswers.has(q.id) && !store.isAnswerCorrect(q.id)
}

/** 用户该题是否未作答 */
function isUnanswered(q: Question): boolean {
  return !store.userAnswers.has(q.id) || !store.userAnswers.get(q.id)
}

/** 该选项是否为正确答案 */
function isOptionCorrectAnswer(q: Question, idx: number): boolean {
  const letter = String.fromCharCode(65 + idx)
  const correct = q.answer
  if (Array.isArray(correct)) return correct.includes(letter)
  return correct === letter
}

/** 该选项是否为用户选错的选项（用户选了但不是正确答案） */
function isOptionWrongSelected(q: Question, idx: number): boolean {
  const letter = String.fromCharCode(65 + idx)
  const userAns = store.userAnswers.get(q.id) || ''
  const parts = userAns.split(',').filter(Boolean)
  if (!parts.includes(letter)) return false
  // 用户选了该项，但该项不是正确答案
  return !isOptionCorrectAnswer(q, idx)
}

/** 审题模式下选项的样式类 */
function getReviewOptionClass(q: Question, idx: number): string {
  const letter = String.fromCharCode(65 + idx)
  const userAns = store.userAnswers.get(q.id) || ''
  const parts = userAns.split(',').filter(Boolean)
  const isSelected = parts.includes(letter)
  const isCorrectOpt = isOptionCorrectAnswer(q, idx)

  if (isCorrectOpt) return 'review-option-correct'
  if (isSelected && !isCorrectOpt) return 'review-option-wrong'
  return ''
}

function onReviewPageChange(page: number) {
  reviewPage.value = page
  const el = document.querySelector('.result-page')
  if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = seconds % 60
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

/** 键盘快捷键：A/B/C/D 快速选择首个未答题目的对应选项 */
function handleKeydown(e: KeyboardEvent) {
  const tag = (e.target as HTMLElement)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return
  if (examSubmitted.value || settingStep.value !== 'exam') return

  // 找到第一个未答的选择题
  const firstUnanswered = store.questions.find(q => {
    if (q.type === 'fill' || q.type === 'judge') return false
    return !store.userAnswers.has(q.id) || !store.userAnswers.get(q.id)
  })
  if (!firstUnanswered) return

  const key = e.key.toLowerCase()
  const optionIndex = key.charCodeAt(0) - 97 // 'a'=0, 'b'=1, ...
  if (optionIndex < 0 || optionIndex >= firstUnanswered.options.length) return

  e.preventDefault()
  selectOption(firstUnanswered.id, optionIndex)
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

          <!-- 打乱选项顺序 -->
          <a-form-item>
            <template #label>
              <span>
                打乱答案顺序
                <a-tooltip title="开启后，多选答案不区分选择顺序（A,B 与 B,A 都判为正确）">
                  <ExclamationCircleOutlined style="color: #999; font-size: 12px" />
                </a-tooltip>
              </span>
            </template>
            <a-switch v-model:checked="store.sortAnswerOrder" />
            <span style="margin-left: 8px; color: #999; font-size: 12px">
              多选答案不区分顺序
            </span>
          </a-form-item>

          <!-- 打乱选项顺序 -->
          <a-form-item>
            <template #label>
              <span>
                打乱选项顺序
                <a-tooltip title="开启后，每道题的选项顺序会被随机打乱，防止背答案">
                  <ExclamationCircleOutlined style="color: #999; font-size: 12px" />
                </a-tooltip>
              </span>
            </template>
            <a-switch v-model:checked="shuffleMode" />
            <span style="margin-left: 8px; color: #999; font-size: 12px">
              随机排列各题选项顺序
            </span>
          </a-form-item>

          <!-- 考试时间 -->
          <a-form-item label="考试时间">
            <div style="display: flex; align-items: center; gap: 12px">
              <a-slider
                v-model:value="examMinutes"
                :min="5"
                :max="examMinutesSliderMax"
                :marks="{ 5: '5分', 30: '30分', 60: '1小时', 120: '2小时' }"
                style="flex: 1; margin: 0"
              />
              <a-input-number
                v-model:value="examMinutes"
                :min="5"
                :max="480"
                style="width: 72px"
              />
              <span style="color: #999; font-size: 12px; white-space: nowrap">分钟</span>
            </div>
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
      <!-- 顶部状态栏：计时器 + 进度 -->
      <div class="exam-header">
        <a-tag color="orange">模拟考试</a-tag>
        <span
          style="flex: 1; text-align: center; font-size: 20px; font-weight: bold; font-family: monospace"
          :style="{ color: remainingSeconds < 60 ? 'red' : remainingSeconds < 180 ? '#fa8c16' : '#333' }"
        >
          ⏱ {{ formatTime(remainingSeconds) }}
        </span>
        <span style="margin-right: 12px">{{ store.answeredCount }} / {{ store.totalCount }} 已答</span>
        <a-dropdown>
          <a-button :style="{ marginLeft: '16px' }">
            <DownOutlined /> 更多
          </a-button>
          <template #overlay>
            <a-menu>
              <a-menu-item>
                <a-space>
                  <a-switch v-model:checked="indeterminateMode" size="small" />
                  <span>不定项</span>
                </a-space>
              </a-menu-item>
              <a-menu-item>
                <a-space>
                  <a-switch v-model:checked="store.sortAnswerOrder" size="small" />
                  <span>打乱答案顺序</span>
                </a-space>
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item>
                <a-space>
                  <a-switch v-model:checked="shuffleMode" size="small" />
                  <span>打乱选项顺序</span>
                </a-space>
              </a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>
        <a-button
          type="primary"
          danger
          :disabled="examSubmitted"
          @click="submitExam(false)"
        >
          交卷
        </a-button>
      </div>

      <a-divider />

      <div v-if="store.totalCount === 0" style="text-align: center; padding: 80px">
        <a-empty description="暂无题目，请返回调整考试设置">
          <a-button type="primary" @click="settingStep = 'settings'">返回设置</a-button>
        </a-empty>
      </div>

      <!-- 答题区域 - 逐题显示（性能优化：避免大数据量下渲染所有题目的 DOM） -->
      <div v-else class="exam-question-area">
        <!-- 当前题目进度 -->
        <div class="exam-progress-row">
          <a-progress
            :percent="Math.round(((store.currentIndex + 1) / store.totalCount) * 100)"
            :show-info="false"
            size="small"
            style="width: 100%"
          />
          <div class="exam-progress-label">
            <span>第 <strong>{{ store.currentIndex + 1 }}</strong> / {{ store.totalCount }} 题</span>
          </div>
        </div>

        <!-- 当前题目 -->
        <div v-if="store.currentQuestion" class="exam-question">
          <div class="question-stem">
            <span class="q-number">{{ store.currentIndex + 1 }}.</span>
            <a-tag v-if="indeterminateMode && isChoiceType(store.currentQuestion.type)" color="orange" size="small" style="margin-right: 6px">
              不定项选择
            </a-tag>
            <a-tag v-else-if="store.currentQuestion.type" :color="typeColors[store.currentQuestion.type]" size="small" style="margin-right: 6px">
              {{ typeLabels[store.currentQuestion.type] || store.currentQuestion.type }}
            </a-tag>
            {{ store.currentQuestion.stem }}
          </div>

          <div class="options-list">
            <div
              v-for="(opt, idx) in (shuffleMode && optionShuffleMaps.get(store.currentQuestion.id))
                ? optionShuffleMaps.get(store.currentQuestion.id)!.map(i => store.currentQuestion.options[i])
                : store.currentQuestion.options"
              :key="shuffleMode && optionShuffleMaps.get(store.currentQuestion.id) ? optionShuffleMaps.get(store.currentQuestion.id)![idx] : idx"
              class="option-item"
              :class="getOptionClass(store.currentQuestion.id, idx)"
              @click="selectOption(store.currentQuestion.id, idx)"
            >
              <span class="option-letter">{{ String.fromCharCode(65 + idx) }}</span>
              <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
            </div>
          </div>
        </div>

        <!-- 底部导航 -->
        <div class="exam-nav-buttons">
          <a-space>
            <a-button
              :disabled="store.currentIndex === 0"
              @click="store.prev()"
            >
              <LeftOutlined /> 上一题
            </a-button>
            <a-button
              v-if="store.currentIndex < store.totalCount - 1"
              type="primary"
              @click="store.next()"
            >
              下一题 <RightOutlined />
            </a-button>
            <a-button
              v-else
              type="primary"
              danger
              @click="submitExam(false)"
            >
              交卷
            </a-button>
          </a-space>
        </div>
      </div>
    </div>

    <!-- ===== 考试结果 + 错题回顾 ===== -->
    <div v-else class="result-page">
      <!-- 成绩总览 -->
      <div class="result-header">
        <div
          class="result-badge"
          :style="{ background: examScore.percentage >= 60 ? '#52c41a' : '#f5222d' }"
        >
          <CheckCircleOutlined v-if="examScore.percentage >= 60" />
          <CloseCircleOutlined v-else />
        </div>
        <div class="result-title" :style="{ color: examScore.percentage >= 60 ? '#52c41a' : '#f5222d' }">
          {{ examScore.percentage >= 60 ? '考试通过！' : '还需努力' }}
        </div>
        <div class="result-score">
          {{ examScore.correct }} / {{ examScore.total }} &nbsp;
          <span :style="{ color: examScore.percentage >= 60 ? '#52c41a' : '#f5222d', fontWeight: 'bold' }">
            （{{ examScore.percentage }}%）
          </span>
        </div>
        <div class="result-actions">
          <a-space>
            <a-button @click="router.push('/')">返回首页</a-button>
            <a-button type="primary" @click="router.go(0)">重新考试</a-button>
          </a-space>
        </div>
      </div>

      <!-- 错题回顾 -->
      <div class="review-section">
        <h3 style="margin-bottom: 12px">
          📝 答题详情
          <span style="font-size: 13px; color: #999; font-weight: normal; margin-left: 8px">
            共 {{ store.totalCount }} 题，答对 {{ store.correctCount }} 题，答错 {{ store.totalCount - store.correctCount }} 题
          </span>
        </h3>

        <!-- 分页控制 -->
        <div v-if="store.totalCount > reviewPageSize" class="review-pagination-bar">
          <a-pagination
            v-model:current="reviewPage"
            :total="store.totalCount"
            :page-size="reviewPageSize"
            :show-total="(total: number) => `共 ${total} 题，第 ${reviewPage} / ${Math.ceil(total / reviewPageSize)} 页`"
            show-size-changer
            :page-size-options="['10', '20', '50', '100']"
            @change="onReviewPageChange"
            size="small"
          />
        </div>

        <div
          v-for="(q, localIdx) in paginatedReviewQuestions"
          :key="q.id"
          class="review-card"
          :class="{ 'review-card-wrong': isWrong(q) || isUnanswered(q) }"
        >
          <div class="review-card-header">
            <span class="review-q-number">{{ (reviewPage - 1) * reviewPageSize + localIdx + 1 }}.</span>
            <a-tag v-if="q.type" :color="typeColors[q.type]" size="small">
              {{ typeLabels[q.type] || q.type }}
            </a-tag>
            <span class="review-stem">{{ q.stem }}</span>
            <span v-if="isUnanswered(q)" class="review-status review-status-unanswered">未作答</span>
            <span v-else-if="store.isAnswerCorrect(q.id)" class="review-status review-status-correct">
              <CheckCircleOutlined /> 正确
            </span>
            <span v-else class="review-status review-status-wrong">
              <CloseCircleOutlined /> 错误
            </span>
          </div>

          <!-- 选择题选项回顾 -->
          <div v-if="q.options.length > 0" class="review-options">
            <div
              v-for="(opt, idx) in q.options"
              :key="idx"
              class="review-option"
              :class="getReviewOptionClass(q, idx)"
            >
              <span class="option-letter">{{ String.fromCharCode(65 + idx) }}</span>
              <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
              <CheckOutlined v-if="isOptionCorrectAnswer(q, idx)" class="option-icon option-icon-correct" />
              <CloseOutlined v-if="isOptionWrongSelected(q, idx)" class="option-icon option-icon-wrong" />
            </div>
          </div>

          <!-- 用户答案 vs 正确答案 -->
          <div class="review-answer-row">
            <div class="review-answer-item">
              <span style="color: #999">你的答案：</span>
              <span
                v-if="isUnanswered(q)"
                style="color: #999; font-style: italic"
              >（未作答）</span>
              <span
                v-else
                :style="{ color: store.isAnswerCorrect(q.id) ? '#52c41a' : '#f5222d', fontWeight: 500 }"
              >
                {{ formatAnswerText(q, store.userAnswers.get(q.id) || '') }}
              </span>
            </div>
            <div v-if="!store.isAnswerCorrect(q.id)" class="review-answer-item">
              <span style="color: #999">正确答案：</span>
              <span style="color: #52c41a; font-weight: 500">
                {{ formatAnswerText(q, Array.isArray(q.answer) ? q.answer.join(',') : q.answer) }}
              </span>
            </div>
          </div>

          <!-- 解析 -->
          <div v-if="q.explanation" class="review-explanation">
            <span style="color: #1890ff; font-weight: 500">💡 解析：</span>
            {{ q.explanation }}
          </div>
        </div>

        <!-- 底部翻页 -->
        <div v-if="store.totalCount > reviewPageSize" class="review-pagination-bar">
          <a-pagination
            v-model:current="reviewPage"
            :total="store.totalCount"
            :page-size="reviewPageSize"
            :show-total="(total: number) => `共 ${total} 题，第 ${reviewPage} / ${Math.ceil(total / reviewPageSize)} 页`"
            show-size-changer
            :page-size-options="['10', '20', '50', '100']"
            size="small"
          />
        </div>
      </div>
    </div>
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

.exam-question-area {
  margin-top: 12px;
}

.exam-progress-row {
  margin-bottom: 20px;
}

.exam-progress-label {
  text-align: center;
  margin-top: 6px;
  font-size: 14px;
  color: #666;
}

.exam-nav-buttons {
  display: flex;
  justify-content: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

.question-stem {
  font-size: 15px;
  line-height: 1.6;
  margin-bottom: 8px;
  white-space: pre-line;
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

/* ===== 考试结果 + 错题回顾 ===== */
.result-page {
  padding-bottom: 40px;
}

.result-header {
  text-align: center;
  padding: 32px 16px;
  background: linear-gradient(135deg, #fafafa 0%, #f0f5ff 100%);
  border-radius: 12px;
  margin-bottom: 24px;
}

.result-badge {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 28px;
  margin-bottom: 12px;
}

.result-title {
  font-size: 24px;
  font-weight: bold;
  margin-bottom: 8px;
}

.result-score {
  font-size: 18px;
  color: #555;
  margin-bottom: 16px;
}

.result-actions {
  margin-top: 8px;
}

.review-section {
  margin-top: 8px;
}

.review-card {
  border: 1px solid #e8e8e8;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
  background: #fff;
  transition: box-shadow 0.2s;
}

.review-card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.review-card-wrong {
  border-left: 4px solid #f5222d;
}

.review-card-header {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 8px;
}

.review-q-number {
  font-weight: bold;
  color: #fa8c16;
  margin-right: 4px;
}

.review-status {
  font-size: 12px;
  font-weight: 500;
  white-space: nowrap;
  margin-left: auto;
}

.review-status-correct {
  color: #52c41a;
}

.review-status-wrong {
  color: #f5222d;
}

.review-status-unanswered {
  color: #999;
}

.review-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 8px 0 8px 24px;
}

.review-option {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border: 1px solid #e8e8e8;
  border-radius: 6px;
  font-size: 13px;
}

.review-option-correct {
  border-color: #b7eb8f !important;
  background: #f6ffed !important;
}

.review-option-correct .option-letter {
  background: #52c41a;
  color: #fff;
}

.review-option-wrong {
  border-color: #ffa39e !important;
  background: #fff2f0 !important;
}

.review-option-wrong .option-letter {
  background: #f5222d;
  color: #fff;
}

.review-option .option-icon {
  margin-left: auto;
  font-size: 14px;
}

.review-option .option-icon-correct {
  color: #52c41a;
}

.review-option .option-icon-wrong {
  color: #f5222d;
}

.review-answer-row {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-top: 8px;
  padding: 8px 12px;
  background: #fafafa;
  border-radius: 6px;
  font-size: 13px;
}

.review-answer-item {
  line-height: 1.6;
}

.review-stem {
  flex: 1;
  margin-left: 6px;
  white-space: pre-line;
}

.review-explanation {
  margin-top: 8px;
  padding: 8px 12px;
  background: #e6f7ff;
  border-radius: 6px;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-line;
}

.review-pagination-bar {
  display: flex;
  justify-content: center;
  margin: 16px 0;
}
</style>
