import { useEffect, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { ChatsCircle, Plus, Trash, CaretRight } from '@phosphor-icons/react'
import { Page } from '../../components/Page'
import { call } from '../../native'
import { useT } from '../../i18n'
import { mocks, ago, currentConversation, setCurrentConversation, type ConvSummary } from '../../data/claude'
import '../Claude.css'

/** /claude/conversaciones: abrir o borrar conversaciones guardadas */
export function ClaudeConversations() {
  const t = useT()
  const navigate = useNavigate()
  const [list, setList] = useState<ConvSummary[] | null>(null)
  useEffect(() => { call<ConvSummary[]>('claude_conversations', {}, mocks.conversations).then(setList).catch(() => setList([])) }, [])
  const fresh = () => { setCurrentConversation(null); navigate('/claude') }

  return (
    <Page title={t('claude.conversations')}>
      <div className="c-top">
        <button className="btn-primary" data-nav="conv-new" data-nav-default={list?.length === 0 ? '' : undefined} onClick={fresh}>
          <Plus size={18} weight="light" /> {t('claude.new')}
        </button>
      </div>
      {list === null && <p className="hero-meta">{t('claude.loading')}</p>}
      {list?.length === 0 && (
        <div className="empty-state">
          <span className="empty-icon"><ChatsCircle size={26} weight="light" /></span>
          <h2>{t('claude.conv.emptyTitle')}</h2>
          <p>{t('claude.conv.emptyText')}</p>
        </div>
      )}
      <div className="c-list">
        {list?.map((c, i) => (
          <div className="c-row" key={c.id}>
            <button className="option" data-nav={`conv-${c.id}`} data-nav-default={i === 0 ? '' : undefined} onClick={() => navigate(`/claude?c=${encodeURIComponent(c.id)}`)}>
              <span className="option-main">
                <span className="row-label">{c.title}</span>
                <span className="row-sub">{ago(c.updatedAt)} · {t('claude.conv.messages', { n: c.count })}</span>
              </span>
              <CaretRight size={18} weight="light" />
            </button>
            <div className="c-actions">
              <button className="c-icon-btn danger" data-nav={`conv-del-${c.id}`} onClick={() => navigate(`/claude/conversaciones/${encodeURIComponent(c.id)}/borrar`)}>
                <Trash size={18} weight="light" /> {t('claude.conv.delete')}
              </button>
            </div>
          </div>
        ))}
      </div>
    </Page>
  )
}

/** /claude/conversaciones/:id/borrar: confirmar antes de borrar */
export function ClaudeConversationDelete() {
  const t = useT()
  const navigate = useNavigate()
  const { id = '' } = useParams()
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    call<{ title: string }>('claude_conversation', { id }, { title: mocks.conversation.title }).then((c) => setTitle(c.title)).catch(() => {})
  }, [id])
  const remove = async () => {
    try {
      await call('claude_conversation_delete', { id })
      if (currentConversation() === id) setCurrentConversation(null)
      navigate('/claude/conversaciones', { replace: true })
    } catch (e) { setError(String(e)) }
  }
  return (
    <Page title={t('claude.del.title')}>
      <div className="c-confirm">
        <h2>{t('claude.del.question')}</h2>
        {title && <p>{t('claude.del.quoted', { title })}</p>}
        <p>{t('claude.del.warning')}</p>
        <div className="hero-actions">
          <button className="btn-ghost" data-nav="del-keep" data-nav-default="" onClick={() => navigate(-1)}>{t('claude.del.keep')}</button>
          <button className="btn-primary" data-nav="del-yes" onClick={remove}><Trash size={18} weight="light" /> {t('claude.conv.delete')}</button>
        </div>
        {error && <p className="notice error">{error}</p>}
      </div>
    </Page>
  )
}
