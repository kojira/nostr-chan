import { Card, CardContent, Typography, Box, Chip, IconButton, Tooltip, Avatar, Dialog, DialogTitle, DialogContent, DialogActions, TextField, Button } from '@mui/material';
import { CheckCircle, Cancel, PlayArrow, Pause, Edit, Delete, SmartToy, Send, Info, Summarize } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import type { BotData } from '../types';
import { useMemo, useState } from 'react';
import { nip19 } from 'nostr-tools';

interface BotCardProps {
  bot: BotData;
  onEdit: (bot: BotData) => void;
  onDelete: (pubkey: string) => void;
  onToggle: (pubkey: string) => void;
}

export const BotCard = ({ bot, onEdit, onDelete, onToggle }: BotCardProps) => {
  const navigate = useNavigate();
  const isActive = bot.status === 0;
  const [postDialogOpen, setPostDialogOpen] = useState(false);
  const [postContent, setPostContent] = useState('');
  const [posting, setPosting] = useState(false);

  // contentからJSONパース
  const kind0Info = useMemo(() => {
    try {
      if (!bot.content) return null;
      return JSON.parse(bot.content);
    } catch {
      return null;
    }
  }, [bot.content]);

  // npub形式に変換
  const npub = useMemo(() => {
    try {
      return nip19.npubEncode(bot.pubkey);
    } catch {
      return null;
    }
  }, [bot.pubkey]);

  const handlePost = async () => {
    if (!postContent.trim()) {
      alert('投稿内容を入力してください');
      return;
    }

    setPosting(true);
    try {
      const response = await fetch(`/api/bots/${bot.pubkey}/post`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: postContent }),
      });

      if (!response.ok) {
        throw new Error('投稿に失敗しました');
      }

      alert('✅ 投稿しました！');
      setPostContent('');
      setPostDialogOpen(false);
    } catch (error) {
      console.error('投稿エラー:', error);
      alert('❌ 投稿に失敗しました');
    } finally {
      setPosting(false);
    }
  };

  return (
    <Card 
      elevation={0}
      sx={{ 
        height: '100%',
        background: '#ffffff',
        border: '1px solid',
        borderColor: isActive ? '#667eea' : '#e5e7eb',
        borderRadius: '16px',
        transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
        position: 'relative',
        overflow: 'hidden',
        boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
        '&:hover': { 
          transform: 'translateY(-8px)', 
          boxShadow: '0 20px 40px rgba(102, 126, 234, 0.25)',
          borderColor: isActive ? '#764ba2' : '#667eea',
        },
        '&::before': isActive ? {
          content: '""',
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          height: '4px',
          background: 'linear-gradient(90deg, #667eea 0%, #764ba2 100%)',
        } : {},
      }}
    >
      <CardContent>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
            <Avatar
              src={kind0Info?.picture || undefined}
              sx={{ 
                width: 72, 
                height: 72,
                border: isActive ? '4px solid #667eea' : '4px solid #e5e7eb',
                boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
              }}
            >
              <SmartToy sx={{ fontSize: 40 }} />
            </Avatar>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              {(kind0Info?.name || kind0Info?.display_name) && (
                <Typography variant="h6" fontWeight="bold" noWrap sx={{ mb: 0.5 }}>
                  {kind0Info.display_name || kind0Info.name}
                </Typography>
              )}
              <Typography 
                variant="caption" 
                fontFamily="monospace" 
                color="text.secondary"
                sx={{ 
                  display: 'block',
                  wordBreak: 'break-all',
                  lineHeight: 1.3,
                }}
              >
                hex: {bot.pubkey}
              </Typography>
              {npub && (
                <Typography 
                  variant="caption" 
                  fontFamily="monospace" 
                  color="text.secondary"
                  sx={{ 
                    display: 'block',
                    wordBreak: 'break-all',
                    lineHeight: 1.3,
                    mt: 0.3,
                  }}
                >
                  npub: {npub}
                </Typography>
              )}
              <Chip
                label={isActive ? '✓ 有効' : '× 無効'}
                size="small"
                color={isActive ? 'success' : 'default'}
                icon={isActive ? <CheckCircle /> : <Cancel />}
                sx={{ 
                  fontWeight: 'bold',
                  fontSize: '0.7rem',
                  mt: 0.5,
                }}
              />
            </Box>
          </Box>
          
          <Box sx={{ display: 'flex', gap: 0.5 }}>
            <Tooltip title="Bot詳細">
              <IconButton 
                onClick={() => navigate(`/bots/${bot.pubkey}`)}
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: 'rgba(33, 150, 243, 0.08)',
                    color: '#2196f3',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                <Info fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="会話要約">
              <IconButton 
                onClick={() => navigate(`/bots/${bot.pubkey}/summaries`)}
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: 'rgba(156, 39, 176, 0.08)',
                    color: '#9c27b0',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                <Summarize fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="投稿">
              <IconButton 
                onClick={() => setPostDialogOpen(true)}
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: 'rgba(2, 136, 209, 0.08)',
                    color: '#0288d1',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                <Send fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title={isActive ? '無効化' : '有効化'}>
              <IconButton 
                onClick={() => onToggle(bot.pubkey)}
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: isActive ? 'rgba(237, 108, 2, 0.08)' : 'rgba(46, 125, 50, 0.08)',
                    color: isActive ? '#ed6c02' : '#2e7d32',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                {isActive ? <Pause fontSize="small" /> : <PlayArrow fontSize="small" />}
              </IconButton>
            </Tooltip>
            <Tooltip title="編集">
              <IconButton 
                onClick={() => onEdit(bot)} 
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: 'rgba(102, 126, 234, 0.08)',
                    color: '#667eea',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                <Edit fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="削除">
              <IconButton 
                onClick={() => onDelete(bot.pubkey)}
                sx={{
                  color: 'text.secondary',
                  bgcolor: 'rgba(0, 0, 0, 0.04)',
                  '&:hover': {
                    bgcolor: 'rgba(211, 47, 47, 0.08)',
                    color: '#d32f2f',
                  },
                  transition: 'all 0.2s',
                }}
                size="small"
              >
                <Delete fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>

        <Box 
          sx={{ 
            borderLeft: '4px solid',
            borderColor: 'primary.main', 
            pl: 2, 
            py: 1,
            mb: 2,
            bgcolor: 'rgba(102, 126, 234, 0.03)',
            borderRadius: '0 8px 8px 0',
          }}
        >
          <Typography variant="caption" fontWeight="bold" color="primary.main" textTransform="uppercase" letterSpacing={0.8}>
            プロンプト
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            display: '-webkit-box',
            WebkitLineClamp: 2,
            WebkitBoxOrient: 'vertical',
            mt: 0.5,
            lineHeight: 1.6,
          }}>
            {bot.prompt}
          </Typography>
        </Box>

        {kind0Info && Object.keys(kind0Info).length > 0 && (
          <Box 
            sx={{ 
              borderLeft: '4px solid',
              borderColor: 'secondary.main', 
              pl: 2,
              py: 1,
              bgcolor: 'rgba(118, 75, 162, 0.03)',
              borderRadius: '0 8px 8px 0',
            }}
          >
            <Typography variant="caption" fontWeight="bold" color="secondary.main" textTransform="uppercase" letterSpacing={0.8}>
              追加情報
            </Typography>
            <Box sx={{ mt: 0.5 }}>
              {Object.entries(kind0Info).map(([key, value]) => (
                <Typography key={key} variant="body2" color="text.secondary" sx={{ mb: 0.5, wordBreak: 'break-word' }}>
                  <strong>{key}:</strong> {String(value)}
                </Typography>
              ))}
            </Box>
          </Box>
        )}
      </CardContent>

      {/* 投稿ダイアログ */}
      <Dialog open={postDialogOpen} onClose={() => setPostDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>📝 {kind0Info?.display_name || kind0Info?.name || 'Bot'}として投稿</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label="投稿内容"
            fullWidth
            multiline
            rows={4}
            value={postContent}
            onChange={(e) => setPostContent(e.target.value)}
            placeholder="投稿したい内容を入力してください..."
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setPostDialogOpen(false)}>
            キャンセル
          </Button>
          <Button onClick={handlePost} variant="contained" disabled={posting || !postContent.trim()}>
            {posting ? '投稿中...' : '投稿'}
          </Button>
        </DialogActions>
      </Dialog>
    </Card>
  );
};

