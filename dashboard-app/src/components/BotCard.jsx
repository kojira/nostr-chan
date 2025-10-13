import { Card, CardContent, Typography, Box, Chip, IconButton, Tooltip } from '@mui/material';
import { CheckCircle, Cancel, PlayArrow, Pause, Edit, Delete } from '@mui/icons-material';

export const BotCard = ({ bot, onEdit, onDelete, onToggle }) => {
  const isActive = bot.status === 0;

  return (
    <Card sx={{ transition: 'all 0.3s', '&:hover': { transform: 'translateY(-2px)', boxShadow: 4 } }}>
      <CardContent>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', mb: 2 }}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, flex: 1 }}>
            {isActive ? (
              <CheckCircle sx={{ fontSize: 40, color: 'success.main' }} />
            ) : (
              <Cancel sx={{ fontSize: 40, color: 'grey.400' }} />
            )}
            <Box>
              <Typography variant="body2" fontFamily="monospace" fontWeight="bold" noWrap>
                {bot.pubkey.substring(0, 24)}...
              </Typography>
              <Chip
                label={isActive ? '🟢 有効' : '⚫ 無効'}
                size="small"
                color={isActive ? 'success' : 'default'}
                sx={{ mt: 0.5 }}
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

