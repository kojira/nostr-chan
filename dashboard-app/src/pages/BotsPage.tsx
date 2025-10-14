import { Container, Box, Typography, IconButton } from '@mui/material';
import { ArrowBack } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { BotsSection } from '../sections/BotsSection';
import { useBots } from '../hooks/useBots';
import { useStats } from '../hooks/useStats';
import { useDailyReplies } from '../hooks/useDailyReplies';

export const BotsPage = () => {
  const navigate = useNavigate();
  const { bots, reload: reloadBots } = useBots();
  const { reload: reloadStats } = useStats();
  const { reload: reloadDailyReplies } = useDailyReplies();

  const handleRefresh = () => {
    reloadBots();
    reloadStats();
    reloadDailyReplies();
  };

  return (
    <Container maxWidth="xl" sx={{ py: 4 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
        <IconButton onClick={() => navigate('/')} sx={{ mr: 2 }}>
          <ArrowBack />
        </IconButton>
        <Typography variant="h4" fontWeight="bold">
          Bot管理
        </Typography>
      </Box>
      
      <BotsSection bots={bots} onRefresh={handleRefresh} />
    </Container>
  );
};

