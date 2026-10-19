'use strict';
const $ = id => document.getElementById(id);
const number = new Intl.NumberFormat('pt-BR');
const percent = new Intl.NumberFormat('pt-BR', {maximumFractionDigits: 1});
let lastData;
let timer;
function text(id, value) {const node = $(id); if (node.textContent !== value) node.textContent = value;}
function duration(seconds) {
  const h = Math.floor(seconds / 3600), m = Math.floor(seconds % 3600 / 60), s = seconds % 60;
  return `${h ? `${h}h ` : ''}${String(m).padStart(2,'0')}m ${String(s).padStart(2,'0')}s`;
}
function render(data) {
  const state = data.pipeline.state;
  const preparing = state === 'waiting_for_build' && data.buildSucceeded && data.pipelineAlive === true;
  const compiling = state === 'waiting_for_build' && !preparing && data.buildAlive !== false && data.pipelineAlive !== false;
  const validating = state === 'running_native_smoke' && data.pipelineAlive !== false;
  const passed = state === 'native_smoke_passed';
  const failed = state === 'failed';
  const stopped = !passed && !failed && !compiling && !validating && !preparing;
  const phase = compiling ? 'Compilando Chromium' : preparing ? 'Preparando os testes' : validating ? 'Validando o NavZip' : passed ? 'Testes nativos aprovados' : failed ? 'Falha registrada' : 'Compilação sem processo ativo';
  text('phase', phase);
  text('connection', stopped ? 'Interrompido' : 'Ao vivo · 2 s');
  $('connection').className = `connection ${stopped ? 'offline' : 'online'}`;
  text('context', `Retomada atual${data.version ? ` · Chromium ${data.version}` : ''}`);
  const progress = data.progress;
  if (progress && progress.totalSteps > 0) {
    const total = progress.totalSteps, done = data.buildSucceeded ? total : progress.completedSteps;
    text('completed', number.format(done));
    text('remaining', number.format(Math.max(0,total-done)));
    text('ratio', `${number.format(done)} de ${number.format(total)} etapas desta retomada`);
    const value = Math.min(100,100*done/total);
    text('percent', `${percent.format(value)}%`);
    $('progress').value = value;
  } else {
    text('completed','—');text('remaining','—');text('ratio','Conferindo os arquivos existentes');text('percent','—');
    $('progress').removeAttribute('value');
  }
  const ended = passed || failed || stopped;
  const clock = ended ? Math.min(data.now,data.pipeline.updatedAtUnix || data.now) : data.now;
  text('elapsed', data.startedAt ? duration(Math.max(0,clock-data.startedAt)) : '—');
  text('updated', `Dados atualizados às ${new Date(data.now*1000).toLocaleTimeString('pt-BR')}`);
  text('disk', data.diskFreeGiB == null ? 'Disco: indisponível' : `${number.format(Math.floor(data.diskFreeGiB))} GiB livres · reserva mínima de 20 GiB`);
  $('build-stage').className = data.buildSucceeded ? 'done' : compiling ? 'active' : failed ? 'failed' : '';
  text('build-stage-state',data.buildSucceeded ? 'Concluída' : compiling ? 'Em andamento' : failed ? 'Ver log' : 'Interrompida');
  $('test-stage').className = validating || preparing ? 'active' : passed ? 'done' : failed && data.buildSucceeded ? 'failed' : '';
  text('test-stage-state',validating ? 'Em andamento' : preparing ? 'Iniciando' : passed ? 'Aprovados' : failed && data.buildSucceeded ? 'Falharam' : 'Aguardando o build');
  const error = data.pipeline.error || (stopped ? 'O acompanhamento não encontrou a compilação ativa. Os arquivos concluídos permanecem disponíveis para retomada.' : '');
  $('error').hidden = !error; text('error', error);
  const lines = (validating || passed || (failed && data.buildSucceeded)) ? data.testLog : data.buildLog;
  const log = $('log'), follow = log.scrollHeight-log.scrollTop-log.clientHeight < 28;
  text('log',lines?.length ? lines.join('\n') : 'Aguardando as próximas etapas…');
  if (follow) log.scrollTop = log.scrollHeight;
}
async function update() {
  clearTimeout(timer);
  try {
    const response = await fetch('/status', {cache:'no-store',signal:AbortSignal.timeout(5000)});
    if (!response.ok) throw new Error('Monitor indisponível');
    lastData = await response.json(); render(lastData);
  } catch {
    text('connection','Sem conexão');$('connection').className = 'connection offline';
    text('updated',lastData ? 'Conexão interrompida · exibindo os últimos dados recebidos' : 'Reconectando ao monitor local…');
  }
  timer = setTimeout(update,document.hidden ? 10000 : 2000);
}
document.addEventListener('visibilitychange',() => {if (!document.hidden) update();});
update();
