# Privacidade

LinMic não possui conta, analytics, publicidade, telemetria ou relay externo. O áudio trafega diretamente entre o telefone e o PC selecionado, com criptografia. Não há gravação no app normal.

O microfone é acessado durante uma conexão iniciada pelo usuário. A notificação permanente permite mutar ou desconectar. Mute silencia o áudio transmitido; encerrar a conexão também encerra a captura. O visualizador usa apenas dados em memória.

A descoberta mDNS anuncia o nome do PC na rede local. O desktop guarda o nome do telefone e um token de associação; o Android guarda endereço do PC, preferências e token protegido pelo Keystore. A lista de dispositivos pareados permite revogar associações. Desinstalar/remove os dados locais, mas não revoga a associação salva no outro dispositivo: esqueça o telefone também no desktop.

APK instalado manualmente exige autorização de instalação no navegador/gerenciador de arquivos. LinMic não solicita acesso a contatos, localização ou armazenamento de mídia. As ferramentas de desenvolvimento podem baixar dependências e os testes de áudio podem criar arquivos temporários quando executados explicitamente.

O Android 0.3.0 mantém no armazenamento privado até 40 eventos técnicos de sessão, sem áudio, códigos, tokens ou endereços. O painel de diagnóstico exibe esses eventos e motivos de encerramento fornecidos pelo Android. O relatório só é copiado mediante ação do usuário; não é enviado automaticamente. Uma sessão explicitamente iniciada pode ser retomada pelo sistema após encerrar o processo; desconectar cancela essa intenção.
