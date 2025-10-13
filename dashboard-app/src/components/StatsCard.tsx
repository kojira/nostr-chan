import { Card, CardContent, Typography, Box } from '@mui/material';
import type { SvgIconComponent } from '@mui/icons-material';

interface StatsCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: SvgIconComponent;
  color?: 'primary' | 'success' | 'info' | 'warning' | 'error';
}

export const StatsCard = ({ title, value, subtitle, icon: Icon, color = 'primary' }: StatsCardProps) => {
  return (
    <Card 
      elevation={0}
      sx={{ 
        height: '100%', 
        transition: 'all 0.3s',
        border: '1px solid',
        borderColor: 'divider',
        background: 'linear-gradient(135deg, #ffffff 0%, #fafafa 100%)',
        '&:hover': { 
          transform: 'translateY(-4px)', 
          boxShadow: '0 8px 24px rgba(0,0,0,0.12)',
          borderColor: `${color}.main`,
        } 
      }}
    >
      <CardContent sx={{ p: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', mb: 3 }}>
          <Box sx={{ flex: 1 }}>
            <Typography 
              variant="caption" 
              color="text.secondary" 
              fontWeight={700} 
              textTransform="uppercase" 
              letterSpacing={1.5} 
              sx={{ display: 'block', mb: 1.5, fontSize: '0.7rem' }}
            >
              {title}
            </Typography>
            <Typography 
              variant="h1" 
              component="div" 
              fontWeight="900" 
              sx={{ 
                lineHeight: 1, 
                fontSize: { xs: '2.5rem', sm: '3rem', md: '3.5rem' },
                background: `linear-gradient(135deg, ${color}.main 0%, ${color}.dark 100%)`,
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
              }}
            >
              {value}
            </Typography>
            {subtitle && (
              <Typography variant="body2" color="text.secondary" sx={{ mt: 1, fontWeight: 500 }}>
                {subtitle}
              </Typography>
            )}
          </Box>
          {Icon && (
            <Box 
              sx={{ 
                width: 56, 
                height: 56, 
                borderRadius: '16px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: `linear-gradient(135deg, ${color}.light 0%, ${color}.main 100%)`,
                boxShadow: `0 4px 12px rgba(0,0,0,0.1)`,
              }}
            >
              <Icon sx={{ fontSize: 30, color: 'white' }} />
            </Box>
          )}
        </Box>
      </CardContent>
    </Card>
  );
};

