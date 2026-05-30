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

/** 练习模式标签 */
export const PracticeModeLabel: Record<PracticeMode, string> = {
  sequential: '顺序练习',
  random: '随机练习',
  wrong: '错题练习',
  exam: '模拟考试',
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
