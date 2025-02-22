/*
   +----------------------------------------------------------------------+
   | HipHop for PHP                                                       |
   +----------------------------------------------------------------------+
   | Copyright (c) 2010-present Facebook, Inc. (http://www.facebook.com)  |
   +----------------------------------------------------------------------+
   | This source path is subject to version 3.01 of the PHP license,      |
   | that is bundled with this package in the path LICENSE, and is        |
   | available through the world-wide-web at the following url:           |
   | http://www.php.net/license/3_01.txt                                  |
   | If you did not receive a copy of the PHP license and are unable to   |
   | obtain it through the world-wide-web, please send a note to          |
   | license@php.net so we can mail you a copy immediately.               |
   +----------------------------------------------------------------------+
*/

#include <memory>

#include <folly/futures/Future.h>
#include <folly/portability/GMock.h>
#include <folly/portability/GTest.h>

#include "folly/executors/GlobalExecutor.h"
#include "hphp/runtime/base/watchman.h"
#include "hphp/runtime/ext/facts/exception.h"
#include "hphp/runtime/ext/facts/file-facts.h"
#include "hphp/runtime/ext/facts/watchman-watcher.h"

using ::testing::ByMove;
using ::testing::ElementsAre;
using ::testing::Return;

namespace HPHP {
namespace Facts {
namespace {

struct MockWatchman final : public Watchman {
  MOCK_METHOD(
      folly::SemiFuture<folly::dynamic>, query, (folly::dynamic), (override));

  MOCK_METHOD(folly::SemiFuture<watchman::Clock>, getClock, (), (override));

  MOCK_METHOD(
      void,
      subscribe,
      (const folly::dynamic& queryObj,
       watchman::SubscriptionCallback&& callback),
      (override));
};

TEST(WatchmanWatcherTest, sinceAndClockArePassedThrough) {
  auto mockWatchman = std::make_shared<MockWatchman>();
  auto watcher = make_watchman_watcher(folly::dynamic::object(), *mockWatchman);

  EXPECT_CALL(*mockWatchman, query)
      .WillOnce(Return(
          ByMove(folly::makeSemiFuture<folly::dynamic>(folly::dynamic::object(
              "clock", "this is the new clock")("is_fresh_instance", false)))));

  auto since = Clock{.m_clock = "this is the old clock"};
  auto result = watcher->getChanges(*folly::getGlobalIOExecutor(), since).get();
  EXPECT_EQ(result.m_lastClock, since);
  EXPECT_EQ(result.m_newClock, Clock{.m_clock = "this is the new clock"});
}

TEST(WatchmanWatcherTest, filesAndExistenceArePassedThrough) {
  auto mockWatchman = std::make_shared<MockWatchman>();
  auto watcher = make_watchman_watcher(folly::dynamic::object(), *mockWatchman);

  EXPECT_CALL(*mockWatchman, query)
      .WillOnce(Return(ByMove(folly::makeSemiFuture<folly::dynamic>(
          folly::dynamic::object("clock", "2")("is_fresh_instance", false)(
              "files",
              folly::dynamic::array(
                  folly::dynamic::object("name", "a.hck")("exists", true)(
                      "content.sha1hex", "faceb00c"),
                  folly::dynamic::object("name", "b.hck")(
                      "exists", false)))))));

  auto results =
      watcher->getChanges(*folly::getGlobalIOExecutor(), Clock{}).get();
  EXPECT_THAT(
      results.m_files,
      ElementsAre(
          Watcher::ResultFile{
              .m_path = "a.hck", .m_exists = true, .m_hash = "faceb00c"},
          Watcher::ResultFile{.m_path = "b.hck", .m_exists = false}));
}

TEST(WatchmanWatcherTest, malformedWatchmanOutput) {
  auto mockWatchman = std::make_shared<MockWatchman>();
  auto watcher = make_watchman_watcher(folly::dynamic::object(), *mockWatchman);

  // No "clock" field
  EXPECT_CALL(*mockWatchman, query)
      .WillOnce(Return(ByMove(
          folly::makeSemiFuture<folly::dynamic>(folly::dynamic::object))));
  EXPECT_THROW(
      watcher->getChanges(*folly::getGlobalIOExecutor(), {}).get(), UpdateExc);

  // "clock" field is an empty object instead of a string
  EXPECT_CALL(*mockWatchman, query)
      .WillOnce(Return(ByMove(folly::makeSemiFuture<folly::dynamic>(
          folly::dynamic::object("clock", folly::dynamic::object)))));
  EXPECT_THROW(
      watcher->getChanges(*folly::getGlobalIOExecutor(), {}).get(), UpdateExc);
}

TEST(WatchmanWatcherTest, querySinceMergebaseIsNotFresh) {
  auto mockWatchman = std::make_shared<MockWatchman>();
  auto watcher = make_watchman_watcher(folly::dynamic::object(), *mockWatchman);

  // If you didn't give Watchman a local clock, Watchman will return
  // `is_fresh_instance: true` even if you gave it a mergebase. Results from
  // these queries are not actually fresh as far as we're concerned.
  EXPECT_CALL(*mockWatchman, query)
      .WillOnce(Return(ByMove(folly::makeSemiFuture<folly::dynamic>(
          folly::dynamic::object("clock", "1")("is_fresh_instance", true)))))
      .WillOnce(Return(ByMove(folly::makeSemiFuture<folly::dynamic>(
          folly::dynamic::object("clock", "2")("is_fresh_instance", true)))));

  // This query is asking for all files in the repo, not since a given point in
  // time or since a given commit. This is actually fresh from our perspective.
  Clock sinceWithoutMergebase;
  auto resultsWithoutMergebase =
      watcher->getChanges(*folly::getGlobalIOExecutor(), Clock{}).get();
  EXPECT_TRUE(resultsWithoutMergebase.m_fresh);

  // This query is based off of a mergebase commit hash, so we don't consider it
  // fresh - Watchman is not returning all the files in the repo, it's returning
  // all the files since a given point in time.
  auto resultsWithMergebase =
      watcher
          ->getChanges(
              *folly::getGlobalIOExecutor(), Clock{.m_mergebase = "faceb00c"})
          .get();
  EXPECT_FALSE(resultsWithMergebase.m_fresh);
}

} // namespace
} // namespace Facts
} // namespace HPHP
