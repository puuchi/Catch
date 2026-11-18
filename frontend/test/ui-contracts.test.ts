import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compileTemplate, parse } from 'vue/compiler-sfc';

test('native tooltip and calendar implementations stay out of shared Vue controls', () => {
  const root = fileURLToPath(new URL('../src/', import.meta.url));
  const violations: string[] = [];
  const files = readdirSync(root, { recursive: true }).filter((file) => file.endsWith('.vue'));
  for (const file of files) {
    const source = readFileSync(join(root, file), 'utf8');
    const template = parse(source).descriptor.template;
    if (!template) continue;
    const result = compileTemplate({
      source: template.content,
      filename: file,
      id: file,
      compilerOptions: {
        nodeTransforms: [
          (node) => {
            // Vue AST uses 1 for elements, 0 for native tags, 6 for attributes and 7 for directives.
            if (node.type !== 1 || node.tagType !== 0) return;
            const location = `${file}:${node.loc.start.line}`;
            const nativeTitle = node.props.some(
              (prop) =>
                (prop.type === 6 && prop.name === 'title') ||
                (prop.type === 7 &&
                  prop.name === 'bind' &&
                  prop.arg?.type === 4 &&
                  prop.arg.content === 'title'),
            );
            if (nativeTitle) violations.push(`${location} uses native title instead of v-tooltip`);
            const iconButton =
              node.tag === 'button' &&
              node.props.some(
                (prop) =>
                  prop.type === 6 &&
                  prop.name === 'class' &&
                  prop.value?.content.split(/\s+/).includes('icon-button'),
              );
            const tooltip = node.props.some((prop) => prop.type === 7 && prop.name === 'tooltip');
            if (iconButton && !tooltip)
              violations.push(`${location} has an icon button without a shared tooltip`);
            const nativeCalendar =
              node.tag === 'input' &&
              node.props.some(
                (prop) =>
                  prop.type === 6 &&
                  prop.name === 'type' &&
                  ['date', 'datetime-local'].includes(prop.value?.content || ''),
              );
            if (nativeCalendar)
              violations.push(`${location} uses a native calendar instead of DateField`);
          },
        ],
      },
    });
    assert.deepEqual(result.errors, [], file);
  }
  assert.deepEqual(violations, []);
});
