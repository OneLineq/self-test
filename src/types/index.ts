// ============================================================
// 刷题助手 — 类型定义
// ============================================================

/** 题库 */
export interface Bank {
  id: string
  name: string
  created_at: string
  question_count: number
}

/** 题目 */
export interface Question {
  id: string
  bank_id: string
  stem: string
  type: QuestionType
  options: string[]
  answer: string | string[]   // 单选 "A"，多选 ["A","C"]，填空 "文本"
  explanation: string
  // 统计
  times_attempted: number
  times_correct: number
  last_attempted: string | null
  // 是否在错题集中
  is_wrong?: boolean
}

/** 题型 */
export type QuestionType = 'single' | 'multiple' | 'judge' | 'fill' | ''

/** 练习记录 */
export interface PracticeRecord {
  id: number
  question_id: string
  bank_id: string
  user_answer: string
  is_correct: boolean
  mode: PracticeMode
  timestamp: string
}

/** 练习模式 */
export type PracticeMode = 'sequential' | 'random' | 'wrong' | 'exam'

/** 判断是否为选择题（单选/多选），即可使用不定项模式 */
export function isChoiceType(type: QuestionType): boolean {
  return type === 'single' || type === 'multiple'
}

const TYPE_ALIASES: Record<string, QuestionType> = {
  '单选题': 'single',
  '单选': 'single',
  '多选题': 'multiple',
  '多选': 'multiple',
  '不定项': 'multiple',
  '不定项选择': 'multiple',
  '多项选择': 'multiple',
  '多项选择题': 'multiple',
  '判断题': 'judge',
  '判断': 'judge',
  '填空题': 'fill',
  '填空': 'fill',
}

/** 从 invoke 返回的原始对象读取题型字段（兼容不同 key 写法） */
export function readQuestionType(raw: unknown): string {
  if (!raw || typeof raw !== 'object') return ''
  const obj = raw as Record<string, unknown>
  for (const key of ['type', 'Type', 'question_type', 'questionType']) {
    const val = obj[key]
    if (typeof val === 'string' && val.trim()) return val.trim()
  }
  return ''
}

/** 答案是否为多选格式（数组或多字母） */
function isMultiAnswer(answer: unknown): boolean {
  if (Array.isArray(answer)) return answer.length > 0
  if (typeof answer === 'string') {
    const cleaned = answer.trim().toUpperCase().replace(/,/g, '').replace(/，/g, '').replace(/\s/g, '')
    return cleaned.length > 1 && /^[A-J]+$/.test(cleaned)
  }
  return false
}

/** 解析题目有效题型：多选答案优先于错误的单选标记，再匹配别名与推断 */
export function resolveQuestionType(
  q: Pick<Question, 'answer' | 'options'> & { type?: unknown },
): QuestionType {
  const raw = readQuestionType(q) || String(q.type ?? '').trim()
  const lower = raw.toLowerCase()

  if (lower === 'judge' || lower === 'fill') return lower
  if (raw && (TYPE_ALIASES[raw] === 'judge' || TYPE_ALIASES[raw] === 'fill')) {
    return TYPE_ALIASES[raw]
  }

  if (isMultiAnswer(q.answer)) return 'multiple'

  if (lower === 'single' || lower === 'multiple') return lower as QuestionType
  if (raw && TYPE_ALIASES[raw]) return TYPE_ALIASES[raw]

  if (typeof q.answer === 'string') {
    const upper = q.answer.trim().toUpperCase()
    const cleaned = upper.replace(/,/g, '').replace(/，/g, '').replace(/\s/g, '')
    if (/^[A-J]$/.test(cleaned)) {
      return q.options.length === 2 ? 'judge' : 'single'
    }
    if (upper) return 'fill'
  }
  return ''
}

/** 规范化题目对象，确保 type 字段可用于展示与分组 */
export function normalizeQuestion(raw: unknown): Question {
  const q = raw as Question
  const explicit = readQuestionType(raw)
  const merged = { ...q, type: (explicit || q.type || '') as QuestionType }
  const resolved = resolveQuestionType(merged)
  return resolved === merged.type ? merged : { ...merged, type: resolved }
}

/** 练习模式标签 */
export const PracticeModeLabel: Record<PracticeMode, string> = {
  sequential: '顺序练习',
  random: '随机练习',
  wrong: '错题练习',
  exam: '模拟考试',
}

/** 练习记忆条目（含题目详情） */
export interface PracticeMemoryItem {
  id: number
  question_id: string
  bank_id: string
  stem: string
  type: QuestionType
  correct_answer: string
  user_answer: string
  is_correct: boolean
  mode: PracticeMode
  timestamp: string
}

/** 题库预览（数据库概览 / 导入预览） */
export interface SourceBankPreview {
  name: string
  question_count: number
  created_at: string
  name_conflict: boolean
}

/** 当前数据库信息 */
export interface DatabaseInfo {
  path: string
  size_bytes: number
  bank_count: number
  question_count: number
  record_count: number
  banks: SourceBankPreview[]
}

/** 外部数据库预览 */
export interface SourceDatabasePreview {
  file_path: string
  size_bytes: number
  bank_count: number
  question_count: number
  record_count: number
  banks: SourceBankPreview[]
}

/** 数据库替换导入结果 */
export interface ImportReplaceResult {
  backup_path: string
}

/** 数据库合并结果 */
export interface MergeResult {
  banks_added: number
  banks_merged: number
  banks_skipped: number
  questions_added: number
  questions_skipped: number
  questions_overwritten: number
  errors: string[]
}

/** Excel 列映射配置 */
export interface ColumnMapping {
  stem_col: number       // 题干列索引 (0-based)
  type_col: number | null // 题型列索引，null 表示无此列
  option_start_col: number // 选项起始列索引
  option_count: number   // 选项列数
  answer_col: number     // 答案列索引
  explanation_col: number | null // 解析列索引
}
