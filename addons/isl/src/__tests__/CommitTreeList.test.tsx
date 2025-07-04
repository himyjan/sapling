/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import {act, fireEvent, render, screen, waitFor, within} from '@testing-library/react';
import App from '../App';
import {CommitInfoTestUtils, ignoreRTL} from '../testQueries';
import {
  closeCommitInfoSidebar,
  COMMIT,
  commitInfoIsOpen,
  expectMessageSentToServer,
  openCommitInfoSidebar,
  resetTestMessages,
  simulateCommits,
  simulateRepoConnected,
  simulateUncommittedChangedFiles,
} from '../testUtils';
import {CommandRunner} from '../types';

describe('CommitTreeList', () => {
  beforeEach(() => {
    resetTestMessages();
  });

  it('shows loading spinner on mount', () => {
    render(<App />);

    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('shows bug button even on error', () => {
    render(<App />);

    act(() => {
      simulateCommits({
        error: new Error('invalid certificate'),
      });
    });

    expect(screen.getByTestId('bug-button')).toBeInTheDocument();
    expect(screen.getByText('invalid certificate')).toBeInTheDocument();
  });

  describe('after commits loaded', () => {
    beforeEach(() => {
      render(<App />);
      act(() => {
        simulateRepoConnected();
        closeCommitInfoSidebar();
        expectMessageSentToServer({
          type: 'subscribe',
          kind: 'smartlogCommits',
          subscriptionID: expect.anything(),
        });
        simulateCommits({
          value: [
            COMMIT('1', 'some public base', '0', {phase: 'public'}),
            COMMIT('a', 'My Commit', '1'),
            COMMIT('b', 'Another Commit', 'a', {isDot: true}),
          ],
        });
      });
    });

    it('renders commits', () => {
      expect(screen.getByText('My Commit')).toBeInTheDocument();
      expect(screen.getByText('Another Commit')).toBeInTheDocument();
      expect(screen.queryByText('some public base')).not.toBeInTheDocument();
    });

    it('renders exactly one head', () => {
      expect(screen.getByText('You are here')).toBeInTheDocument();
    });

    describe('uncommitted changes', () => {
      beforeEach(() => {
        act(() => {
          expectMessageSentToServer({
            type: 'subscribe',
            kind: 'uncommittedChanges',
            subscriptionID: expect.anything(),
          });
          simulateUncommittedChangedFiles({
            value: [
              {path: 'src/file.js', status: 'M'},
              {path: 'src/file_add.js', status: 'A'},
              {path: 'src/file_removed.js', status: 'R'},
              {path: 'src/file_untracked.js', status: '?'},
              {path: 'src/file_missing.js', status: '!'},
            ],
          });
        });
      });

      it('renders uncommitted changes', () => {
        expect(screen.getByText(ignoreRTL('file.js'), {exact: false})).toBeInTheDocument();
        expect(screen.getByText(ignoreRTL('file_add.js'), {exact: false})).toBeInTheDocument();
        expect(screen.getByText(ignoreRTL('file_removed.js'), {exact: false})).toBeInTheDocument();
        expect(
          screen.getByText(ignoreRTL('file_untracked.js'), {exact: false}),
        ).toBeInTheDocument();
        expect(screen.getByText(ignoreRTL('file_missing.js'), {exact: false})).toBeInTheDocument();
      });

      it('shows quick commit button', () => {
        expect(screen.getByText('Commit')).toBeInTheDocument();
      });
      it('shows quick amend button only on non-public commits', () => {
        expect(screen.getByText('Amend')).toBeInTheDocument();
        // checkout a public commit
        act(() => {
          simulateCommits({
            value: [
              COMMIT('1', 'some public base', '0', {phase: 'public', isDot: true}),
              COMMIT('a', 'My Commit', '1', {successorInfo: {hash: 'a2', type: 'land'}}),
              COMMIT('b', 'Another Commit', 'a'),
            ],
          });
        });
        // no longer see quick amend
        expect(screen.queryByText('Amend')).not.toBeInTheDocument();
      });

      it('shows file actions', () => {
        const fileActions = screen.getAllByTestId('file-actions');
        expect(fileActions).toHaveLength(5); // 5 files
        const revertButtons = screen.getAllByTestId('file-revert-button');
        expect(revertButtons).toHaveLength(3); // modified, removed, missing files can be reverted
      });

      it('runs revert command when clicking revert button', async () => {
        const revertButtons = screen.getAllByTestId('file-revert-button');
        jest.spyOn(window, 'confirm').mockImplementation(() => true);
        act(() => {
          fireEvent.click(revertButtons[0]);
        });
        await waitFor(() => {
          expect(window.confirm).toHaveBeenCalled();
          expectMessageSentToServer({
            type: 'runOperation',
            operation: {
              args: ['revert', {type: 'repo-relative-file-list', paths: ['src/file.js']}],
              id: expect.anything(),
              runner: CommandRunner.Sapling,
              trackEventName: 'RevertOperation',
            },
          });
        });
      });

      describe('addremove', () => {
        it('hides addremove if all files tracked', () => {
          act(() => {
            simulateUncommittedChangedFiles({
              value: [
                {path: 'src/file.js', status: 'M'},
                {path: 'src/file_add.js', status: 'A'},
                {path: 'src/file_removed.js', status: 'R'},
              ],
            });
          });
          expect(screen.queryByTestId('addremove-button')).not.toBeInTheDocument();

          act(() => {
            simulateUncommittedChangedFiles({
              value: [
                {path: 'src/file.js', status: 'M'},
                {path: 'src/file_add.js', status: 'A'},
                {path: 'src/file_removed.js', status: 'R'},
                {path: 'src/file_untracked.js', status: '?'},
                {path: 'src/file_missing.js', status: '!'},
              ],
            });
          });
          expect(screen.queryByTestId('addremove-button')).toBeInTheDocument();
        });

        it('runs addremove', async () => {
          const addremove = screen.getByTestId('addremove-button');
          act(() => {
            fireEvent.click(addremove);
          });
          await waitFor(() => {
            expectMessageSentToServer({
              type: 'runOperation',
              operation: {
                args: ['addremove'],
                id: expect.anything(),
                runner: CommandRunner.Sapling,
                trackEventName: 'AddRemoveOperation',
              },
            });
          });
        });

        it('optimistically updates file statuses while addremove is running', async () => {
          const addremove = screen.getByTestId('addremove-button');
          act(() => {
            fireEvent.click(addremove);
          });
          await waitFor(() => {
            expectMessageSentToServer({
              type: 'runOperation',
              operation: {
                args: ['addremove'],
                id: expect.anything(),
                runner: CommandRunner.Sapling,
                trackEventName: 'AddRemoveOperation',
              },
            });
          });

          expect(
            document.querySelectorAll('.changed-files .changed-file.file-ignored'),
          ).toHaveLength(0);
        });

        it('runs addremove only on selected files that are untracked', async () => {
          const ignoredFileCheckboxes = document.querySelectorAll(
            '.changed-files .changed-file.file-ignored input[type="checkbox"]',
          );
          const missingFileCheckboxes = document.querySelectorAll(
            '.changed-files .changed-file.file-missing input[type="checkbox"]',
          );
          expect(ignoredFileCheckboxes).toHaveLength(1); // file_untracked.js
          expect(missingFileCheckboxes).toHaveLength(1); // file_missing.js
          act(() => {
            fireEvent.click(missingFileCheckboxes[0]);
          });

          const addremove = screen.getByTestId('addremove-button');
          act(() => {
            fireEvent.click(addremove);
          });
          await waitFor(() => {
            expectMessageSentToServer({
              type: 'runOperation',
              operation: {
                // note: although src/file.js & others are selected, they aren't passed to addremove as they aren't untracked
                args: ['addremove', {path: 'src/file_untracked.js', type: 'repo-relative-file'}],
                id: expect.anything(),
                runner: CommandRunner.Sapling,
                trackEventName: 'AddRemoveOperation',
              },
            });
          });
        });
      });
    });

    it('shows log errors', () => {
      act(() => {
        simulateCommits({
          error: new Error('error running log'),
        });
      });
      expect(screen.getByText('Failed to fetch commits')).toBeInTheDocument();
      expect(screen.getByText('error running log')).toBeInTheDocument();

      // we should still have commits from the last successful fetch
      expect(screen.getByText('My Commit')).toBeInTheDocument();
      expect(screen.getByText('Another Commit')).toBeInTheDocument();
      expect(screen.queryByText('some public base')).not.toBeInTheDocument();
    });

    it('shows status errors', () => {
      act(() => {
        simulateUncommittedChangedFiles({
          error: new Error('error running status'),
        });
      });
      expect(screen.getByText('Failed to fetch Uncommitted Changes')).toBeInTheDocument();
      expect(screen.getByText('error running status')).toBeInTheDocument();
    });

    it('shows successor info', () => {
      act(() => {
        simulateCommits({
          value: [
            COMMIT('1', 'some public base', '0', {phase: 'public'}),
            COMMIT('a', 'My Commit', '1', {successorInfo: {hash: 'a2', type: 'land'}}),
            COMMIT('b', 'Another Commit', 'a', {isDot: true}),
          ],
        });
      });
      expect(screen.getByText('Landed as a newer commit', {exact: false})).toBeInTheDocument();
    });

    it('shows button to open sidebar', () => {
      act(() => {
        simulateCommits({
          value: [
            COMMIT('1', 'some public base', '0', {phase: 'public'}),
            COMMIT('a', 'Commit A', '1', {isDot: true}),
            COMMIT('b', 'Commit B', '1'),
          ],
        });
      });
      expect(commitInfoIsOpen()).toBeFalsy();

      // doesn't appear for public commits
      expect(
        within(screen.getByTestId('commit-1')).queryByTestId('open-commit-info-button'),
      ).not.toBeInTheDocument();

      const openButton = within(screen.getByTestId('commit-b')).getByTestId(
        'open-commit-info-button',
      );
      expect(openButton).toBeInTheDocument();
      // screen reader accessible
      expect(screen.getByLabelText('Open commit "Commit B"')).toBeInTheDocument();
      fireEvent.click(openButton);
      expect(commitInfoIsOpen()).toBeTruthy();
      expect(CommitInfoTestUtils.withinCommitInfo().getByText('Commit B')).toBeInTheDocument();
    });

    it('sets to amend mode when clicking open button', () => {
      act(() => {
        simulateCommits({
          value: [
            COMMIT('1', 'some public base', '0', {phase: 'public'}),
            COMMIT('a', 'Commit A', '1', {isDot: true}),
            COMMIT('b', 'Commit B', '1'),
          ],
        });
      });
      act(() => {
        openCommitInfoSidebar();
      });
      CommitInfoTestUtils.clickCommitMode();

      const openButton = within(screen.getByTestId('commit-b')).getByTestId(
        'open-commit-info-button',
      );
      expect(openButton).toBeInTheDocument();
      fireEvent.click(openButton);
      expect(commitInfoIsOpen()).toBeTruthy();
      expect(CommitInfoTestUtils.withinCommitInfo().getByText('Commit B')).toBeInTheDocument();
    });
  });

  describe('render dag subset', () => {
    describe('irrelevant cwd stacks', () => {
      beforeEach(() => {
        render(<App />);
        act(() => {
          // Set repo root to "/repo" and cwd to "/repo/www"
          simulateRepoConnected('/repo', '/repo/www');
          closeCommitInfoSidebar();
          expectMessageSentToServer({
            type: 'subscribe',
            kind: 'smartlogCommits',
            subscriptionID: expect.anything(),
          });
          simulateCommits({
            value: [
              COMMIT('1', 'Public base', '0', {phase: 'public'}),
              // Commits with different maxCommonPathPrefix values
              COMMIT('a', 'Commit in www', '1', {maxCommonPathPrefix: 'www/'}),
              COMMIT('b', 'Commit in addons', '1', {maxCommonPathPrefix: 'addons/'}),
              COMMIT('c', 'Commit in root', '1', {maxCommonPathPrefix: ''}),
              COMMIT('d', 'Commit in www/js', '1', {
                maxCommonPathPrefix: 'www/js/',
                isDot: true,
              }),
            ],
          });
        });
      });

      it('shows irrelevant commits by default', () => {
        // By default, hideIrrelevantCwdStacks is false, so all commits should be visible
        expect(screen.queryByText('Commit in www')).toBeInTheDocument();
        expect(screen.queryByText('Commit in addons')).toBeInTheDocument();
        expect(screen.queryByText('Commit in root')).toBeInTheDocument();
        expect(screen.queryByText('Commit in www/js')).toBeInTheDocument();

        // But the addons/ commit should be marked as irrelevant
        expect(document.body.querySelectorAll('.commit.irrelevant')).toHaveLength(1);
      });

      it('can hide irrelevant commits when enabled', async () => {
        fireEvent.click(screen.getByTestId('settings-gear-button'));

        const settingsDropdown = screen.getByTestId('settings-dropdown');
        expect(settingsDropdown).toBeInTheDocument();
        const cwdIrrelevantDropdown =
          within(settingsDropdown).getByTestId('cwd-irrelevant-commits');
        expect(cwdIrrelevantDropdown).toBeInTheDocument();

        // Change the dropdown value to 'hide'
        fireEvent.change(cwdIrrelevantDropdown, {target: {value: 'hide'}});

        expect(screen.queryByText('Commit in www')).toBeInTheDocument();
        expect(screen.queryByText('Commit in root')).toBeInTheDocument();
        expect(screen.queryByText('Commit in www/js')).toBeInTheDocument();

        expect(screen.queryByText('Commit in addons')).not.toBeInTheDocument();
      });
    });

    describe('obsolete stacks', () => {
      beforeEach(() => {
        render(<App />);
        act(() => {
          simulateRepoConnected();
          closeCommitInfoSidebar();
          expectMessageSentToServer({
            type: 'subscribe',
            kind: 'smartlogCommits',
            subscriptionID: expect.anything(),
          });
          simulateCommits({
            value: [
              COMMIT('1', 'some public base', '0', {phase: 'public'}),
              COMMIT('a', 'Commit A', '1', {successorInfo: {hash: 'a2', type: 'rebase'}}),
              COMMIT('b', 'Commit B', 'a', {successorInfo: {hash: 'b2', type: 'rebase'}}),
              COMMIT('c', 'Commit C', 'b', {successorInfo: {hash: 'c2', type: 'rebase'}}),
              COMMIT('d', 'Commit D', 'c', {successorInfo: {hash: 'd2', type: 'rebase'}}),
              COMMIT('e', 'Commit E', 'd', {isDot: true}),
            ],
          });
        });
      });
      it('hides obsolete stacks by default', () => {
        expect(screen.queryByText('Commit A')).toBeInTheDocument();
        expect(screen.queryByText('Commit B')).not.toBeInTheDocument();
        expect(screen.queryByText('Commit C')).not.toBeInTheDocument();
        expect(screen.queryByText('Commit D')).toBeInTheDocument();
        expect(screen.queryByText('Commit E')).toBeInTheDocument();
      });

      it('can configure to not hide obsolete stacks', () => {
        fireEvent.click(screen.getByTestId('settings-gear-button'));
        fireEvent.click(screen.getByTestId('condense-obsolete-stacks'));
        expect(screen.queryByText('Commit A')).toBeInTheDocument();
        expect(screen.queryByText('Commit B')).toBeInTheDocument();
        expect(screen.queryByText('Commit C')).toBeInTheDocument();
        expect(screen.queryByText('Commit D')).toBeInTheDocument();
        expect(screen.queryByText('Commit E')).toBeInTheDocument();
      });
    });
  });
});
