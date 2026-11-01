# 03 - Regras de Negócio: EventRent

## 1. Princípios Gerais de Negócio
As regras de negócio do EventRent operam de maneira determinística, protegidas por transações atômicas no backend Rust (`crates/core`).

## 2. Regras Fundamentais do Domínio

### 2.1. Controle Híbrido de Estoque (Fases Futuras)
- Cada modelo de equipamento (`equipment_model`) possui a propriedade `control_mode`:
  - `serialized`: Controle individualizado por número de série e patrimônio. Cada item possui histórico de manutenção, horas de uso e estado físico.
  - `quantity`: Controle por quantidade e saldo disponível em estoque. Indicado para cabos, adaptadores, ganchos e consumíveis.
- **Kits**: Agrupamentos lógicos de equipamentos (ex.: Kit Iluminação Palco com 8 refletores + mesa DMX + cabeamento). O kit expande em seus componentes na fase de separação e reserva.

### 2.2. Motor de Disponibilidade e Reserva (Fase 3)
- A verificação de disponibilidade e a efetivação da reserva de equipamentos ocorrem obrigatoriamente na **mesma transação de banco de dados**.
- Não é permitido split-brain ou conferência prévia desvinculada da gravação, eliminando integralmente condições de corrida (*race conditions*) e *overbooking*.
- O cálculo de disponibilidade considera períodos de trânsito, montagem, evento, desmontagem e margem de segurança para manutenção.

### 2.3. Identidade Visual e White Label
- O sistema permite que cada locadora configure seu branding visual.
- A paleta de cores do tenant sobrescreve dinamicamente as variáveis de tema no `:root` sem que nenhum componente precise de código condicional.
- O tema respeita alternância entre modo claro e modo escuro, assegurando que o contraste atenda aos critérios WCAG AA.
