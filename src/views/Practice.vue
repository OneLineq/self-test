<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 练习页面（顺序/随机/错题）
// ============================================================
import { onMounted, onBeforeUnmount, ref, computed, watch, h } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  LeftOutlined,
  RightOutlined,
  CheckOutlined,
  CloseOutlined,
  MinusOutlined,
  ExclamationCircleOutlined,
  DownOutlined,
  HistoryOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
} from '@ant-design/icons-vue'
import { usePracticeStore } from '../stores/practice'
import { usePreferencesStore } from '../stores/preferences'
import { invoke } from '@tauri-apps/api/tauri'
import { PracticeModeLabel, isChoiceType, isMultiAnswer, loadTopicFilter, questionMatchesTopic } from '../types'
import type { PracticeMode, PracticeMemoryItem } from '../types'

const route = useRoute()
const router = useRouter()
const store = usePracticeStore()
const prefs = usePreferencesStore()

const bankId = route.params.bankId as string
const mode = (route.query.mode as PracticeMode) || 'sequential'
const isTopicPractice = route.query.topic === '1'
const selectedAnswer = ref('')
/** 不定项模式：选择题不区分单选/多选，可自由选择一项或多项后提交 */
const indeterminateMode = ref(prefs.indeterminateMode)
/** 打乱选项顺序模式 */
const shuffleMode = ref(prefs.shuffleOptions)
/** 当前题目的选项排列映射：displayIdx → originalIdx */
const optionShuffleMap = ref<number[]>([])

// 从首选项同步「打乱答案顺序」默认值（仅进入时一次，当场改不写回）
store.sortAnswerOrder = prefs.sortAnswerOrder

/** 当前练习中已标记为错题的题目 ID 集合（用于 toggle 按钮显示） */
const wrongSet = ref<Set<string>>(new Set())

/** 生成随机排列（Fisher-Yates） */
function generateShuffle(n: number): number[] {
  const indices = Array.from({ length: n }, (_, i) => i)
  for (let i = n - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [indices[i], indices[j]] = [indices[j], indices[i]]
  }
  return indices
}
const jumpInput = ref<number>(1)

// ===== 刷题记忆 =====
const questionMemory = ref<Map<string, PracticeMemoryItem[]>>(new Map())
const memoryExpanded = ref(false)
const loadingMemory = ref(false)
const clearingMemory = ref(false)

/** 获取当前题目的历史记录 */
async function fetchQuestionMemory() {
  const q = question.value
  if (!q) return
  if (questionMemory.value.has(q.id)) {
    // 已缓存，直接展开
    memoryExpanded.value = !memoryExpanded.value
    return
  }
  loadingMemory.value = true
  try {
    const records = await invoke<PracticeMemoryItem[]>('get_question_memory', { questionId: q.id })
    questionMemory.value.set(q.id, records)
    memoryExpanded.value = true
  } catch (e) {
    message.error('加载记忆失败')
  } finally {
    loadingMemory.value = false
  }
}

/** 清除当前题目的记忆 */
async function clearQuestionMemory() {
  const q = question.value
  if (!q) return
  Modal.confirm({
    title: '清除本题记忆',
    icon: h(ExclamationCircleOutlined),
    content: '确定要清除本题的所有刷题记录吗？此操作不可恢复。',
    okText: '清除',
    okType: 'danger',
    async onOk() {
      clearingMemory.value = true
      try {
        await invoke('clear_question_memory', { questionId: q.id })
        questionMemory.value.delete(q.id)
        memoryExpanded.value = false
        // 更新题目上的统计字段
        q.times_attempted = 0
        q.times_correct = 0
        q.last_attempted = null
        message.success('已清除本题记忆')
      } catch (e) {
        message.error('清除失败: ' + e)
      } finally {
        clearingMemory.value = false
      }
    },
  })
}

/** 解析正确答案用于显示 */
function formatCorrectAnswer(item: PracticeMemoryItem): string {
  if (!item.correct_answer) return ''
  if (item.correct_answer === '""') return '(空)'
  try {
    const parsed = JSON.parse(item.correct_answer)
    if (Array.isArray(parsed)) return parsed.join(', ')
    return parsed || '(空)'
  } catch {
    return item.correct_answer
  }
}

/** 解析用户答案 */
function formatUserAnswer(item: PracticeMemoryItem): string {
  if (!item.user_answer) return '(未作答)'
  return item.user_answer
}
// ===== 断点续练（仅顺序练习，后端 SQLite 持久化） =====
/** 初始化阶段标志：在读取/处理断点之前，防止 watch 误存 currentIndex=0 */
const progressLoaded = ref(false)

async function saveProgress() {
  if (mode !== 'sequential' || isTopicPractice) return
  try {
    await invoke('save_practice_progress', {
      bankId,
      currentIndex: store.currentIndex,
      selectedAnswer: selectedAnswer.value,
    })
  } catch (e) {
    // 静默失败，不影响练习
  }
}

interface PracticeProgress {
  current_index: number
  selected_answer: string
  updated_at: string
}

async function loadProgress(): Promise<number | null> {
  if (mode !== 'sequential') return null
  try {
    const progress = await invoke<PracticeProgress | null>('load_practice_progress', { bankId })
    if (!progress) return null
    const idx = progress.current_index
    if (idx < 0) return null
    return idx
  } catch {
    return null
  }
}

/** 读取已保存的选中答案（仅用户选"继续"时恢复） */
async function loadSavedAnswer(): Promise<string> {
  try {
    const progress = await invoke<PracticeProgress | null>('load_practice_progress', { bankId })
    if (progress && progress.selected_answer) {
      return progress.selected_answer
    }
  } catch {
    // 静默
  }
  return ''
}

/** 键盘快捷键处理 */
function handleKeydown(e: KeyboardEvent) {
  // 输入框中不拦截
  const tag = (e.target as HTMLElement)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return

  const q = question.value
  if (!q) return

  switch (e.key) {
    case 'ArrowLeft':
    case 'ArrowUp':
      e.preventDefault()
      handlePrev()
      break
    case 'ArrowRight':
    case 'ArrowDown':
      e.preventDefault()
      handleNext()
      break
    case 'a':
    case 'A':
      e.preventDefault()
      if (q.options.length >= 1) selectOption(0)
      break
    case 'b':
    case 'B':
      e.preventDefault()
      if (q.options.length >= 2) selectOption(1)
      break
    case 'c':
    case 'C':
      e.preventDefault()
      if (q.options.length >= 3) selectOption(2)
      break
    case 'd':
    case 'D':
      e.preventDefault()
      if (q.options.length >= 4) selectOption(3)
      break
    case 'Enter':
      if (
        selectedAnswer.value &&
        (isMultiAnswer(q.answer) || (indeterminateMode.value && isChoiceType(q.type)) || q.type === 'fill')
      ) {
        e.preventDefault()
        submitMulti()
      }
      break
  }
}

onMounted(async () => {
  window.addEventListener('keydown', handleKeydown)
  try {
    // 主题随机：先按顺序拉全量再过滤打乱，避免后端随机后再过滤题量不准
    const loadMode: PracticeMode =
      isTopicPractice && mode === 'random' ? 'sequential' : mode
    await store.loadQuestions(bankId, loadMode)

    if (isTopicPractice) {
      const filter = loadTopicFilter(bankId)
      if (!filter) {
        message.warning('未找到主题筛选条件，请重新设置')
        router.replace(`/topic/${bankId}`)
        return
      }
      const filtered = store.questions.filter(q =>
        questionMatchesTopic(q, filter.raw, filter.regex, filter.match),
      )
      if (filtered.length === 0) {
        message.warning('没有匹配的题目')
        router.replace(`/topic/${bankId}`)
        return
      }
      store.setQuestions(filtered)
      if (mode === 'random') {
        store.shuffleQuestionOrder()
      }
      store.mode = mode
    }

    // 加载该题库中已有的错题 ID，用于 toggle 按钮初始状态
    if (mode !== 'wrong') {
      try {
        const ids = await invoke<string[]>('list_wrong_question_ids', { bankId })
        wrongSet.value = new Set(ids)
      } catch (_) { /* 静默 */ }
    }
    // 顺序练习：检测断点（主题练习题集已变，不走全库断点）
    if (mode === 'sequential' && !isTopicPractice) {
      const savedIdx = await loadProgress()
      if (savedIdx !== null && savedIdx < store.totalCount && savedIdx > 0) {
        // 有断点，弹窗询问
        await new Promise<void>((resolve) => {
          Modal.confirm({
            title: '发现上次练习进度',
            icon: h(ExclamationCircleOutlined),
            content: `上次练习到第 ${savedIdx + 1} 题（共 ${store.totalCount} 题）`,
            okText: '继续练习',
            cancelText: '从头开始',
            onOk: async () => {
              store.goTo(savedIdx)
              const savedAnswer = await loadSavedAnswer()
              if (savedAnswer) {
                selectedAnswer.value = savedAnswer
              } else {
                const q = store.currentQuestion
                if (q) {
                  selectedAnswer.value = store.userAnswers.get(q.id) || ''
                }
              }
              resolve()
            },
            onCancel: async () => {
              // 从头开始：清除保存的进度
              try {
                await invoke('clear_practice_progress', { bankId })
              } catch {
                // 静默
              }
              store.goTo(0)
              selectedAnswer.value = ''
              resolve()
            },
          })
        })
      }
    }
  } catch (e) {
    message.error('加载题目失败')
    router.back()
  } finally {
    // 无论是否有断点、用户选什么，初始化完成，允许后续 watch 保存进度
    progressLoaded.value = true
  }
})

onBeforeUnmount(async () => {
  window.removeEventListener('keydown', handleKeydown)
  await saveProgress()
})

// 离页确认：防止侧边栏误触退出
// 如果侧边栏已弹过确认框（skipLeaveConfirm），则直接放行
onBeforeRouteLeave((_to, _from, next) => {
  if (store.skipLeaveConfirm) {
    store.skipLeaveConfirm = false
    next()
    return
  }
  if (store.answeredCount > 0) {
    Modal.confirm({
      title: '退出练习？',
      icon: h(ExclamationCircleOutlined),
      content: `已作答 ${store.answeredCount} / ${store.totalCount} 题，退出不会丢失已保存的记录。下次可继续接续练习。`,
      okText: '退出',
      cancelText: '继续答题',
      onOk: () => {
        saveProgress().then(() => next())
      },
      onCancel: () => next(false),
    })
  } else {
    saveProgress().then(() => next())
  }
})

const question = computed(() => store.currentQuestion)

/** 打乱后的选项列表（供模板渲染使用） */
const displayOptions = computed(() => {
  if (!question.value) return []
  if (shuffleMode.value && optionShuffleMap.value.length > 0) {
    return optionShuffleMap.value.map(i => question.value!.options[i])
  }
  return question.value.options
})

/** 打乱模式下，获取显示位置对应的原始选项字母 */
function getOriginalLetter(displayIdx: number): string {
  const originalIdx = shuffleMode.value ? optionShuffleMap.value[displayIdx] : displayIdx
  return String.fromCharCode(65 + originalIdx)
}

// 监听打乱开关切换或题目切换，重新生成排列
watch([shuffleMode, question], () => {
  if (shuffleMode.value && question.value && question.value.options.length > 0) {
    optionShuffleMap.value = generateShuffle(question.value.options.length)
  } else {
    optionShuffleMap.value = []
  }
}, { immediate: true })

function selectOption(optIndex: number) {
  if (!question.value) return
  const qid = question.value.id
  if (store.showResult.get(qid)) return // already answered

  const letter = getOriginalLetter(optIndex)
  const isChoice = question.value ? isChoiceType(question.value.type) : false
  const isMulti = (indeterminateMode.value && isChoice) || isMultiAnswer(question.value?.answer)

  if (isMulti) {
    // 不定项 / 多选题：切换选中
    let current = selectedAnswer.value || ''
    const parts = current ? current.split(',').filter(Boolean) : []
    if (parts.includes(letter)) {
      selectedAnswer.value = parts.filter(p => p !== letter).join(',')
    } else {
      parts.push(letter)
      selectedAnswer.value = parts.join(',')
    }
  } else {
    // 单选题：替换
    selectedAnswer.value = letter
    // 单选题直接提交
    store.submitAnswer(qid, letter)
    syncWrongAfterSubmit(qid)
    // 正确则自动跳转下一题
    if (store.isAnswerCorrect(qid) && store.currentIndex < store.totalCount - 1) {
      setTimeout(() => handleNext(), 400)
    }
  }
}

function getOptionClass(optIndex: number): string {
  if (!question.value) return ''
  const qid = question.value.id

  // 获取原始索引（打乱模式下需映射）
  const originalIdx = shuffleMode.value ? optionShuffleMap.value[optIndex] : optIndex
  const letter = String.fromCharCode(65 + originalIdx)

  // 已提交后的结果展示：用 store 中的答案
  if (store.showResult.get(qid)) {
    const isCorrect = store.isOptionCorrect(question.value, originalIdx)
    const saved = store.userAnswers.get(qid) || ''
    const isSelected = saved === letter || saved.split(',').filter(Boolean).includes(letter)
    if (isCorrect && isSelected) {
      // 多选题且顺序敏感时，检查选项位置
      const answer = question.value.answer
      if (Array.isArray(answer) && !store.sortAnswerOrder) {
        const userParts = saved.split(',').filter(Boolean)
        const correctPos = answer.indexOf(letter)
        const userPos = userParts.indexOf(letter)
        if (correctPos !== userPos) return 'option-missed'
      }
      return 'option-correct'
    }
    if (isCorrect && !isSelected) {
      // 单选/判断：未选中的正确答案用绿色；多选漏选仍用橙色
      const t = question.value.type
      if (t === 'single' || t === 'judge') return 'option-correct'
      return 'option-missed'
    }
    if (isSelected && !isCorrect) return 'option-wrong'
    return ''
  }

  // 未提交时：展示当前本地选中状态
  const local = selectedAnswer.value || ''
  const isLocalSelected = local === letter || local.split(',').filter(Boolean).includes(letter)
  return isLocalSelected ? 'option-selected' : ''
}

// 多选提交
function submitMulti() {
  if (!question.value || !selectedAnswer.value) return
  const qid = question.value.id
  store.submitAnswer(qid, selectedAnswer.value)
  syncWrongAfterSubmit(qid)
  // 正确则自动跳转下一题
  if (store.isAnswerCorrect(qid) && store.currentIndex < store.totalCount - 1) {
    setTimeout(() => handleNext(), 400)
  }
}

function handleNext() {
  selectedAnswer.value = ''
  store.next()
}

function handlePrev() {
  selectedAnswer.value = ''
  store.prev()
}

// Watch for question changes to reset selection and memory，保存断点
watch(() => store.currentIndex, () => {
  const q = store.currentQuestion
  if (q) {
    selectedAnswer.value = store.userAnswers.get(q.id) || ''
  }
  memoryExpanded.value = false
  if (progressLoaded.value) {
    saveProgress()
  }
})

function handleJump() {
  const idx = jumpInput.value - 1
  if (idx >= 0 && idx < store.totalCount) {
    selectedAnswer.value = store.userAnswers.get(store.questions[idx]?.id) || ''
    store.goTo(idx)
  }
}

function exitPractice() {
  Modal.confirm({
    title: '退出练习',
    icon: h(ExclamationCircleOutlined),
    content: '确定要退出当前练习吗？已作答的题目记录不会丢失，下次可继续接续练习。',
    okText: '退出',
    cancelText: '继续练习',
    onOk: async () => {
      await saveProgress()
      store.skipLeaveConfirm = true
      router.push('/')
    },
  })
}

/** 答错后立刻把按钮切成「移出错题集」（与后端错题集规则一致） */
function syncWrongAfterSubmit(qid: string) {
  if (store.isAnswerCorrect(qid)) return
  setWrongFlag(qid, true)
}

function setWrongFlag(qid: string, inSet: boolean) {
  const next = new Set(wrongSet.value)
  if (inSet) next.add(qid)
  else next.delete(qid)
  wrongSet.value = next
}

const isMarkedWrong = computed(() => {
  const q = question.value
  return !!q && wrongSet.value.has(q.id)
})

/** 切换当前题的错题标记状态 */
async function toggleWrong() {
  const q = question.value
  if (!q) return
  const wasIn = wrongSet.value.has(q.id)
  setWrongFlag(q.id, !wasIn)
  try {
    if (wasIn) {
      await invoke('remove_from_wrong', { questionId: q.id })
      message.success('已移出错题集')
    } else {
      await invoke('mark_question_wrong', { questionId: q.id, bankId })
      message.success('已标记为错题')
    }
  } catch (e) {
    setWrongFlag(q.id, wasIn)
    message.error((wasIn ? '移出失败: ' : '标记失败: ') + e)
  }
}

/** 从错题集中移除当前题（错题模式专用，带确认弹窗和自动跳转） */
async function removeWrong() {
  const q = question.value
  if (!q) return
  Modal.confirm({
    title: '移出错题集',
    icon: h(ExclamationCircleOutlined),
    content: `确定将本题「${q.stem.slice(0, 50)}${q.stem.length > 50 ? '...' : ''}」移出错题集吗？`,
    okText: '移出',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('remove_from_wrong', { questionId: q.id })
        message.success('已移出错题集')
        // 如果还有下一题则跳转，否则回到首页
        if (store.currentIndex < store.totalCount - 1) {
          selectedAnswer.value = ''
          store.next()
        } else if (store.currentIndex > 0) {
          store.prev()
        } else {
          router.push('/')
        }
      } catch (e) {
        message.error('移出失败: ' + e)
      }
    },
  })
}

const progress = computed(() =>
  store.totalCount > 0 ? Math.round((store.currentIndex / store.totalCount) * 100) : 0
)

// 题号按钮颜色：正确绿、错误红、未答灰
function navDotStyle(idx: number): Record<string, string> {
  const q = store.questions[idx]
  if (!q || !store.showResult.get(q.id)) return {}
  const correct = store.isAnswerCorrect(q.id)
  return {
    background: correct ? '#52c41a' : '#f5222d',
    color: '#fff',
    borderColor: correct ? '#52c41a' : '#f5222d',
  }
}
</script>

<template>
  <div class="practice-page">
    <!-- 顶部信息栏 -->
    <div class="practice-header">
      <a-space>
        <a-tag color="blue">{{ PracticeModeLabel[mode] }}</a-tag>
        <a-tag v-if="isTopicPractice" color="cyan">主题筛选</a-tag>
        <span>{{ store.currentIndex + 1 }} / {{ store.totalCount }}</span>
        <span style="color: #52c41a">✓ {{ store.correctCount }}</span>
      </a-space>
      <a-progress :percent="progress" :show-info="false" style="flex: 1; margin: 0 16px" />
      <a-dropdown>
        <a-button>
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
      <a-button @click="exitPractice">退出</a-button>
    </div>

    <a-divider />

    <!-- 题目区域 -->
    <div v-if="store.loading" style="text-align: center; padding: 80px">
      <a-spin size="large" />
    </div>

    <div v-else-if="!question" style="text-align: center; padding: 80px">
      <a-empty :description="mode === 'wrong' ? '暂无错题，太棒了！' : '题库为空'">
        <a-button type="primary" @click="router.push('/')">返回首页</a-button>
      </a-empty>
    </div>

    <div v-else class="question-area">
      <!-- 题型标签（不定项模式隐藏单选/多选标签） -->
      <a-tag v-if="indeterminateMode && isChoiceType(question.type)" color="orange" style="margin-bottom: 12px">
        不定项选择
      </a-tag>
      <a-tag v-else-if="question.type" style="margin-bottom: 12px">
        {{ question.type === 'single' ? '单选题' : question.type === 'multiple' ? '多选题' : question.type === 'judge' ? '判断题' : question.type === 'fill' ? '填空题' : question.type }}
      </a-tag>

      <!-- 题干 -->
      <div class="question-stem">
        <span class="q-number">{{ store.currentIndex + 1 }}.</span>
        {{ question.stem }}
      </div>

      <!-- 刷题记忆 -->
      <div class="memory-panel">
        <div class="memory-header" @click="fetchQuestionMemory">
          <a-space>
            <HistoryOutlined :style="{ color: question.times_attempted > 0 ? '#1890ff' : '#ccc' }" />
            <span :style="{ color: question.times_attempted > 0 ? '#333' : '#999', fontWeight: 500 }">
              刷题记忆
            </span>
            <template v-if="question.times_attempted > 0">
              <span style="color: #999; font-size: 12px">
                练过 {{ question.times_attempted }} 次 ·
                正确 {{ question.times_correct }} 次 ·
                正确率 {{ Math.round(question.times_correct / question.times_attempted * 100) }}%
              </span>
              <a-tag :color="question.times_correct === question.times_attempted ? 'green' : 'orange'" style="font-size: 11px">
                {{ question.times_correct === question.times_attempted ? '已掌握' : '需复习' }}
              </a-tag>
            </template>
            <span v-else style="color: #ccc; font-size: 12px">尚未练过此题</span>
          </a-space>
          <a-space>
            <a-button
              v-if="question.times_attempted > 0 && !store.showResult.get(question.id)"
              size="small"
              danger
              type="text"
              :loading="clearingMemory"
              @click.stop="clearQuestionMemory"
            >
              <DeleteOutlined /> 清除
            </a-button>
            <span style="color: #999; font-size: 12px">
              <template v-if="loadingMemory"><a-spin size="small" /></template>
              <template v-else>{{ memoryExpanded ? '收起' : '展开' }}</template>
            </span>
          </a-space>
        </div>

        <!-- 展开的记忆详情 -->
        <div v-if="memoryExpanded" class="memory-body">
          <div v-if="!questionMemory.get(question.id) || questionMemory.get(question.id)!.length === 0" style="text-align: center; padding: 12px; color: #999; font-size: 13px">
            暂无历史记录
          </div>
          <div v-else class="memory-records">
            <div
              v-for="(record, idx) in questionMemory.get(question.id)"
              :key="record.id"
              class="memory-record-item"
            >
              <div class="memory-record-line">
                <span style="color: #999; font-size: 11px; min-width: 20px">{{ idx + 1 }}.</span>
                <span :style="{ color: record.is_correct ? '#52c41a' : '#f5222d', fontWeight: 500, fontSize: 13 }">
                  <span v-if="record.is_correct"><CheckCircleOutlined /> 正确</span>
                  <span v-else><CloseCircleOutlined /> 错误</span>
                </span>
                <span v-if="!record.is_correct" style="font-size: 12px">
                  <span style="color: #999">你的答案：</span>
                  <span style="color: #f5222d">{{ formatUserAnswer(record) }}</span>
                  <span style="color: #999"> 正确答案：</span>
                  <span style="color: #52c41a">{{ formatCorrectAnswer(record) }}</span>
                </span>
                <span style="flex: 1" />
                <span style="color: #bbb; font-size: 11px">{{ record.timestamp?.slice(0, 16).replace('T', ' ') }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 选项 -->
      <div class="options-list">
        <div
          v-for="(opt, displayIdx) in displayOptions"
          :key="shuffleMode ? optionShuffleMap[displayIdx] : displayIdx"
          class="option-item"
          :class="getOptionClass(displayIdx)"
          @click="selectOption(displayIdx)"
        >
          <span class="option-letter">{{ String.fromCharCode(65 + displayIdx) }}</span>
          <span class="option-text">{{ opt.replace(/^[A-D][.、]\s*/, '') }}</span>
          <CheckOutlined v-if="getOptionClass(displayIdx) === 'option-correct'" class="option-icon" />
          <CloseOutlined v-if="getOptionClass(displayIdx) === 'option-wrong'" class="option-icon" />
          <MinusOutlined v-if="getOptionClass(displayIdx) === 'option-missed'" class="option-icon" />
        </div>
      </div>

      <!-- 提交按钮（多选题 或 不定项模式下已选时的选择题） -->
      <div
        v-if="
          !store.showResult.get(question.id) &&
          selectedAnswer &&
          (isMultiAnswer(question.answer) || (indeterminateMode && isChoiceType(question.type)))
        "
        style="text-align: center; margin-top: 16px"
      >
        <a-button type="primary" size="large" @click="submitMulti">
          确认答案（已选 {{ selectedAnswer.split(',').filter(Boolean).length }} 项）
        </a-button>
      </div>

      <!-- 解析 -->
      <a-alert
        v-if="question.explanation && store.showResult.get(question.id)"
        class="explanation-alert"
        message="解析"
        :description="question.explanation"
        type="info"
        show-icon
        style="margin-top: 16px"
      />

      <!-- 底部导航 -->
      <div class="practice-footer">
        <a-button :disabled="store.currentIndex === 0" @click="handlePrev">
          <LeftOutlined /> 上一题
        </a-button>
        <a-space>
          <a-button
            v-if="mode !== 'wrong'"
            size="small"
            :type="isMarkedWrong ? 'primary' : 'default'"
            :danger="!isMarkedWrong"
            :style="isMarkedWrong ? { background: '#f5222d', borderColor: '#f5222d', color: '#fff' } : {}"
            @click="toggleWrong"
          >
            {{ isMarkedWrong ? '移出错题集' : '标记为错题' }}
          </a-button>
          <a-button
            v-if="mode === 'wrong'"
            size="small"
            danger
            @click="removeWrong"
          >
            <DeleteOutlined /> 移出错题集
          </a-button>
        </a-space>
        <span style="color: #999; font-size: 13px">
          {{ store.currentIndex + 1 }} / {{ store.totalCount }}
        </span>
        <a-button
          type="primary"
          :disabled="store.currentIndex >= store.totalCount - 1"
          @click="handleNext"
        >
          下一题 <RightOutlined />
        </a-button>
      </div>

      <!-- 缩略题号导航（仅显示当前附近 + 首尾） -->
      <div class="question-nav-strip">
        <a-button
          v-if="store.currentIndex > 2"
          size="small"
          class="nav-dot"
          :style="navDotStyle(0)"
          @click="store.goTo(0)"
        >1</a-button>
        <span v-if="store.currentIndex > 3" class="nav-ellipsis">…</span>

        <template v-for="offset in [-2, -1, 0, 1, 2]" :key="'offset-' + offset">
          <a-button
            v-if="store.questions[store.currentIndex + offset]"
            size="small"
            class="nav-dot"
            :type="offset === 0 ? 'primary' : 'default'"
            :style="offset !== 0 ? navDotStyle(store.currentIndex + offset) : {}"
            @click="store.goTo(store.currentIndex + offset)"
          >
            {{ store.currentIndex + offset + 1 }}
          </a-button>
        </template>

        <span
          v-if="store.currentIndex < store.totalCount - 4"
          class="nav-ellipsis"
        >…</span>
        <a-button
          v-if="store.currentIndex < store.totalCount - 3"
          size="small"
          class="nav-dot"
          :style="navDotStyle(store.totalCount - 1)"
          @click="store.goTo(store.totalCount - 1)"
        >
          {{ store.totalCount }}
        </a-button>
      </div>

      <!-- 跳转输入 -->
      <div style="text-align: center; margin-top: 8px">
        <a-space>
          <span style="font-size: 12px; color: #999">跳转到</span>
          <a-input-number
            v-model:value="jumpInput"
            :min="1"
            :max="store.totalCount"
            style="width: 70px"
            size="small"
            @pressEnter="handleJump"
          />
          <a-button size="small" @click="handleJump">题</a-button>
        </a-space>
      </div>
    </div>
  </div>
</template>

<style scoped>
.practice-page {
  max-width: 900px;
  margin: 0 auto;
}

.practice-header {
  display: flex;
  align-items: center;
}

.question-stem {
  font-size: 16px;
  line-height: 1.8;
  margin-bottom: 16px;
  padding: 16px;
  background: #fafafa;
  border-radius: 8px;
  border-left: 4px solid #1890ff;
  white-space: pre-line;
}

.explanation-alert :deep(.ant-alert-description) {
  white-space: pre-line;
}

.q-number {
  font-weight: bold;
  color: #1890ff;
  margin-right: 8px;
}

.question-stats {
  font-size: 12px;
  color: #999;
  margin-bottom: 16px;
  padding: 8px 12px;
  background: #f5f5f5;
  border-radius: 4px;
}

.options-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.option-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border: 1px solid #d9d9d9;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.option-item:hover {
  border-color: #1890ff;
  background: #e6f7ff;
}

.option-letter {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #f0f0f0;
  font-weight: bold;
  margin-right: 12px;
  flex-shrink: 0;
}

.option-text {
  flex: 1;
}

.option-icon {
  margin-left: 8px;
  font-size: 18px;
}

.option-correct {
  border-color: #52c41a !important;
  background: #f6ffed !important;
}

.option-correct .option-letter {
  background: #52c41a;
  color: #fff;
}

.option-correct .option-icon {
  color: #52c41a;
}

.option-missed {
  border-color: #fa8c16 !important;
  background: #fff7e6 !important;
}

.option-missed .option-letter {
  background: #fa8c16;
  color: #fff;
}

.option-missed .option-icon {
  color: #fa8c16;
}

.option-wrong {
  border-color: #f5222d !important;
  background: #fff2f0 !important;
}

.option-wrong .option-letter {
  background: #f5222d;
  color: #fff;
}

.option-wrong .option-icon {
  color: #f5222d;
}

.option-selected {
  border-color: #1890ff !important;
  background: #e6f7ff !important;
}

.option-selected .option-letter {
  background: #1890ff;
  color: #fff;
}

.practice-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #f0f0f0;
}

.question-nav-strip {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 4px;
  margin-top: 10px;
}

.nav-dot {
  min-width: 28px;
  height: 28px;
  padding: 0 4px;
  font-size: 12px;
  border-radius: 4px;
}

.nav-ellipsis {
  color: #999;
  font-size: 14px;
  padding: 0 2px;
  user-select: none;
}

/* ===== 刷题记忆面板 ===== */
.memory-panel {
  margin-bottom: 16px;
  border: 1px solid #f0f0f0;
  border-radius: 6px;
  overflow: hidden;
  transition: border-color 0.2s;
}

.memory-panel:hover {
  border-color: #d9d9d9;
}

.memory-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  cursor: pointer;
  user-select: none;
  background: #fafafa;
  transition: background 0.2s;
}

.memory-header:hover {
  background: #f0f5ff;
}

.memory-body {
  border-top: 1px solid #f0f0f0;
  padding: 8px 12px;
  background: #fff;
}

.memory-records {
  max-height: 240px;
  overflow-y: auto;
}

.memory-record-item {
  padding: 4px 0;
}

.memory-record-line {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
}

.memory-record-line:hover {
  background: #fafafa;
}
</style>
