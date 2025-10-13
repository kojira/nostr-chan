import { Card, CardContent, Typography, Box, Chip, IconButton, Tooltip } from '@mui/material';
import { CheckCircle, Cancel, PlayArrow, Pause, Edit, Delete } from '@mui/icons-material';
import type { BotData } from '../types';

interface BotCardProps {
  bot: BotData;
  onEdit: (bot: BotData) => void;
  onDelete: (pubkey: string) => void;
  onToggle: (pubkey: string) => void;
}

export const BotCard = ({ bot, onEdit, onDelete, onToggle }: BotCardProps) => {
  const isActive = bot.status === 0;

  return (
    <Card 
      sx={{ 
        height: '100%',
        background: isActive 
          ? 'linear-gradient(135deg, #667eea15 0%, #764ba215 100%)'
          : 'linear-gradient(135deg, #f5f5f5 0%, #e0e0e0 100%)',
        border: isActive ? '2px solid #667eea' : '2px solid #e0e0e0',
        transition: 'all 0.3s',
        '&:hover': { 
          transform: 'translateY(-4px)', 
          boxShadow: 8,
          border: isActive ? '2px solid #764ba2' : '2px solid #bdbdbd',
        } 
      }}
    >
      <CardContent>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
            {isActive ? (
              <CheckCircle sx={{ fontSize: 48, color: 'success.main' }} />
            ) : (
              <Cancel sx={{ fontSize: 48, color: 'grey.500' }} />
            )}
            <Box>
              <Typography variant="body1" fontFamily="monospace" fontWeight="bold" noWrap sx={{ mb: 0.5 }}>
                {bot.pubkey.substring(0, 20)}...
              </Typography>
              <Chip
                label={isActive ? '✓ 有効' : '× 無効'}
                size="small"
                color={isActive ? 'success' : 'default'}
                sx={{ 
                  fontWeight: 'bold',
                  fontSize: '0.75rem',
                }}
              />
            </Box>
          </Box>
          
          <Box sx={{ display: 'flex', gap: 1 }}>
            <Tooltip title={isActive ? '無効化' : '有効化'}>
              <IconButton 
                onClick={() => onToggle(bot.pubkey)}
                color={isActive ? 'warning' : 'success'}
              >
                {isActive ? <Pause /> : <PlayArrow />}
              </IconButton>
            </Tooltip>
            <Tooltip title="編集">
              <IconButton onClick={() => onEdit(bot)} color="primary">
                <Edit />
              </IconButton>
            </Tooltip>
            <Tooltip title="削除">
              <IconButton onClick={() => onDelete(bot.pubkey)} color="error">
                <Delete />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>

        <Box sx={{ borderLeft: 4, borderColor: 'primary.main', pl: 2, mb: 2 }}>
          <Typography variant="subtitle2" fontWeight="bold" gutterBottom>
            プロンプト
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            display: '-webkit-box',
            WebkitLineClamp: 2,
            WebkitBoxOrient: 'vertical',
          }}>
            {bot.prompt}
          </Typography>
        </Box>

        {bot.content && (
          <Box sx={{ borderLeft: 4, borderColor: 'secondary.main', pl: 2 }}>
            <Typography variant="subtitle2" fontWeight="bold" gutterBottom>
              追加情報
            </Typography>
            <Typography variant="body2" color="text.secondary" noWrap>
              {bot.content}
            </Typography>
          </Box>
        )}
      </CardContent>
    </Card>
  );
};

