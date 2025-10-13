import { Card, CardContent, Typography, Box, Chip, IconButton, Tooltip, Avatar } from '@mui/material';
import { CheckCircle, Cancel, PlayArrow, Pause, Edit, Delete, SmartToy } from '@mui/icons-material';
import type { BotData } from '../types';
import { useMemo } from 'react';

interface BotCardProps {
  bot: BotData;
  onEdit: (bot: BotData) => void;
  onDelete: (pubkey: string) => void;
  onToggle: (pubkey: string) => void;
}

export const BotCard = ({ bot, onEdit, onDelete, onToggle }: BotCardProps) => {
  const isActive = bot.status === 0;

  // contentからJSONパース
  const kind0Info = useMemo(() => {
    try {
      if (!bot.content) return null;
      const parsed = JSON.parse(bot.content);
      return {
        name: parsed.name || parsed.display_name || null,
        picture: parsed.picture || null,
      };
    } catch {
      return null;
    }
  }, [bot.content]);

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
              {kind0Info?.name && (
                <Typography variant="h6" fontWeight="bold" noWrap sx={{ mb: 0.5 }}>
                  {kind0Info.name}
                </Typography>
              )}
              <Typography 
                variant="caption" 
                fontFamily="monospace" 
                color="text.secondary"
                noWrap
                sx={{ display: 'block' }}
              >
                {bot.pubkey.substring(0, 16)}...
              </Typography>
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

        {bot.content && kind0Info?.name && (
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
            <Typography variant="body2" color="text.secondary" noWrap sx={{ mt: 0.5 }}>
              {kind0Info.name}の設定情報
            </Typography>
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

