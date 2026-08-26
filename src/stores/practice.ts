// ============================================================
// 理论训练考核系统 — 练习 Store (Pinia)
// ============================================================
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Question, PracticeMode } from '../types'
import { normalizeQuestion } from '../types'

/** 多选答案比较（支持排序模式） */
function matchAnswer(correct: string[], userAnswer: string, sortFirst: boolean): boolean {
  if (sortFirst) {
    const sorted = [...correct].sort().join(',')
    const userSorted = userAnswer.split(',').filter(Boolean).sort().join(',')
    return sorted === userSorted
  }
  return correct.join(',') === userAnswer
}

export const usePracticeStore = defineStore('practice', () => {
  const questions = ref<Question[]>([])
  const currentIndex = ref(0)
  const mode = ref<PracticeMode>('sequential')
  const userAnswers = ref<Map<string, string>>(new Map())
  const showResult = ref<Map<string, boolean>>(new Map())
  const loading = ref(false)
  /** 侧边栏已确认退出，子页面路由守卫跳过 */
  const skipLeaveConfirm = ref(false)

  /** 多选答案不区分顺序（A,B === B,A）；默认与首选项一致为开 */
  const sortAnswerOrder = ref(true)

  /** 当前题目 */
  const currentQuestion = computed(() => questions.value[currentIndex.value] ?? null)

  /** 总题数 */
  const totalCount = computed(() => questions.value.length)

  /** 已做题数 */
  const answeredCount = computed(() => userAnswers.value.size)

  /** 正确数 */
  const correctCount = computed(() => {
    let count = 0
    userAnswers.value.forEach((answer, qid) => {
      const q = questions.value.find(q => q.id === qid)
      if (q) {
        const correct = Array.isArray(q.answer)
          ? matchAnswer(q.answer, answer, sortAnswerOrder.value)
          : q.answer === answer
        if (correct) count++
      }
    })
    return count
  })

  /** 加载题目 */
  async function loadQuestions(
    bankId: string,
    practiceMode: PracticeMode,
    questionTypes?: string[],
    limit?: number,
    perTypeLimits?: Record<string, number>,
  ) {
    loading.value = true
    mode.value = practiceMode
    currentIndex.value = 0
    userAnswers.value = new Map()
    showResult.value = new Map()
    try {
      const raw = await invoke<Question[]>('get_practice_questions', {
        bankId,
        mode: practiceMode,
        questionTypes: questionTypes && questionTypes.length > 0 ? questionTypes : null,
        limit: limit ?? null,
        perTypeLimits: perTypeLimits && Object.keys(perTypeLimits).length > 0 ? perTypeLimits : null,
      })
      questions.value = raw.map(q => normalizeQuestion(q))
    } finally {
      loading.value = false
    }
  }

  /** 随机打乱题目顺序（模拟考试不定项模式用） */
  function shuffleQuestionOrder() {
    const arr = [...questions.value]
    for (let i = arr.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [arr[i], arr[j]] = [arr[j], arr[i]]
    }
    questions.value = arr
    currentIndex.value = 0
  }

  /** 替换当前题目列表（主题筛选后使用） */
  function setQuestions(list: Question[]) {
    questions.value = list
    currentIndex.value = 0
    userAnswers.value = new Map()
    showResult.value = new Map()
  }

  /** 提交答案 */
  async function submitAnswer(questionId: string, answer: string) {
    userAnswers.value.set(questionId, answer)
    showResult.value.set(questionId, true)

    const q = questions.value.find(q => q.id === questionId)
    if (!q) return

    const correct = Array.isArray(q.answer)
      ? matchAnswer(q.answer, answer, sortAnswerOrder.value)
      : q.answer === answer

    await invoke('record_practice', {
      questionId,
      bankId: q.bank_id,
      userAnswer: answer,
      isCorrect: correct,
      mode: mode.value,
    })
  }

  /** 记录做题（模拟考试交卷用） */
  async function submitExamAnswers() {
    for (const [qid, answer] of userAnswers.value.entries()) {
      const q = questions.value.find(q => q.id === qid)
      if (!q) continue
      const correct = Array.isArray(q.answer)
          ? matchAnswer(q.answer, answer, sortAnswerOrder.value)
          : q.answer === answer
      await invoke('record_practice', {
        questionId: qid,
        bankId: q.bank_id,
        userAnswer: answer,
        isCorrect: correct,
        mode: 'exam',
      })
    }
  }

  /** 下一题 */
  function next() {
    if (currentIndex.value < questions.value.length - 1) {
      currentIndex.value++
    }
  }

  /** 上一题 */
  function prev() {
    if (currentIndex.value > 0) {
      currentIndex.value--
    }
  }

  /** 跳转到指定题 */
  function goTo(index: number) {
    if (index >= 0 && index < questions.value.length) {
      currentIndex.value = index
    }
  }

  /** 判断选项是否正确 */
  function isOptionCorrect(question: Question, optionIndex: number): boolean {
    const correctAnswer = question.answer
    const optionLetter = String.fromCharCode(65 + optionIndex) // A, B, C, D...
    if (Array.isArray(correctAnswer)) {
      return correctAnswer.includes(optionLetter)
    }
    return correctAnswer === optionLetter
  }

  /** 判断用户选择是否正确 */
  function isAnswerCorrect(questionId: string): boolean {
    const q = questions.value.find(q => q.id === questionId)
    const ans = userAnswers.value.get(questionId)
    if (!q || ans === undefined) return false
    if (Array.isArray(q.answer)) {
      return matchAnswer(q.answer, ans, sortAnswerOrder.value)
    }
    return q.answer === ans
  }

  /** 获取用户的选项字母列表 */
  function getUserSelectedLetters(answer: string): string[] {
    return answer.split(',').filter(Boolean)
  }

  /** 获取错题ID列表 */
  function getWrongQuestionIds(): string[] {
    const wrong: string[] = []
    for (const q of questions.value) {
      const ans = userAnswers.value.get(q.id)
      if (ans !== undefined && !isAnswerCorrect(q.id)) {
        wrong.push(q.id)
      }
    }
    return wrong
  }

  return {
    questions,
    currentIndex,
    mode,
    userAnswers,
    showResult,
    loading,
    skipLeaveConfirm,
    currentQuestion,
    totalCount,
    answeredCount,
    correctCount,
    loadQuestions,
    shuffleQuestionOrder,
    setQuestions,
    sortAnswerOrder,
    submitAnswer,
    submitExamAnswers,
    next,
    prev,
    goTo,
    isOptionCorrect,
    isAnswerCorrect,
    getUserSelectedLetters,
    getWrongQuestionIds,
  }
})
