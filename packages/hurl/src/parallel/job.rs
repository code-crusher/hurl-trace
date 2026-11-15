/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use hurl_core::input::Input;
use hurl_core::types::Count;

use crate::runner::{HurlResult, RunnerOptions, Value, VariableSet};
use crate::util::logger::LoggerOptions;

/// A variable to inject in the jobs, in parallel mode.
///
/// A job variable is a variable that is assigned a different value for each job, picked from a
/// list of values. When a job's sequence number is beyond the number of provided values, the
/// values are recycled from the beginning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobVariable {
    /// Name of the variable.
    pub name: String,
    /// The values to cycle through, one for each job.
    pub values: Vec<Value>,
}

impl JobVariable {
    /// Creates a new job variable named `name` with the given `values`.
    pub fn new(name: &str, values: Vec<Value>) -> Self {
        JobVariable {
            name: name.to_string(),
            values,
        }
    }
}

/// Represents the job to run. A job instance groups the input data to execute, and has no methods
/// associated to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    /// The Hurl source content
    pub filename: Input,
    /// The options to run this file.
    pub runner_options: RunnerOptions,
    /// Set of variables injected in the Hurl file
    pub variables: VariableSet,
    /// The logger options for this run
    pub logger_options: LoggerOptions,
    /// The job 0-based index in the jobs list
    pub seq: usize,
}

impl Job {
    /// Creates a new job.
    pub fn new(
        filename: &Input,
        seq: usize,
        runner_options: &RunnerOptions,
        variables: &VariableSet,
        logger_options: &LoggerOptions,
    ) -> Self {
        Job {
            filename: filename.clone(),
            runner_options: runner_options.clone(),
            variables: variables.clone(),
            logger_options: logger_options.clone(),
            seq,
        }
    }
}

pub struct JobResult {
    /// The job corresponding to this job result.
    pub job: Job,
    /// The source content of the job.
    pub content: String,
    /// The result of execution of the job.
    pub hurl_result: HurlResult,
}

impl JobResult {
    /// Creates a new job result.
    pub fn new(job: Job, content: String, hurl_result: HurlResult) -> Self {
        JobResult {
            job,
            content,
            hurl_result,
        }
    }
}

/// A job queue to manage a queue of [`Job`].
///
/// The job queue implements [`Iterator`] trait, and can return a new job to use each time its
/// `next` method is called. This queue can repeat its input sequence a certain number of times, or
/// can loop forever.
///
/// The queue can be created with a list of [`JobVariable`]. Each emitted job gets a copy of the input jobs
/// with the job variables injected, the value of a job variable for a given job is picked from the
/// job variable values, using the job sequence number to cycle through the values (the value at
/// index `seq % values.len()` is used).
pub struct JobQueue<'job> {
    /// The input jobs list.
    jobs: &'job [Job],
    /// Current index of the job, referencing the input job list.
    index: usize,
    /// Repeat mode of this queue (finite or infinite).
    repeat: Count,
    /// Current index of the repeat.
    repeat_index: usize,
    /// The job variables to inject in each job, one value per job.
    job_variables: Vec<JobVariable>,
}

impl<'job> JobQueue<'job> {
    /// Create a new queue, with a list of `jobs` and a `repeat` mode.
    pub fn new(jobs: &'job [Job], repeat: Count) -> Self {
        JobQueue::with_job_variables(jobs, repeat, Vec::new())
    }

    /// Create a new queue, with a list of `jobs`, a `repeat` mode and a list of `job_variables`.
    ///
    /// Job variables are injected in each job, cycling through the values based on the job
    /// sequence number.
    pub fn with_job_variables(
        jobs: &'job [Job],
        repeat: Count,
        job_variables: Vec<JobVariable>,
    ) -> Self {
        JobQueue {
            jobs,
            index: 0,
            repeat,
            repeat_index: 0,
            job_variables,
        }
    }

    /// Returns the effective total number of jobs.
    pub fn jobs_count(&self) -> Count {
        match self.repeat {
            Count::Finite(n) => Count::Finite(self.jobs.len() * n),
            Count::Infinite => Count::Infinite,
        }
    }

    /// Returns a new job at the given `index`.
    fn job_at(&self, index: usize) -> Job {
        let mut job = self.jobs[index].clone();
        // When we're repeating a sequence, we clone an original job and give it a proper
        // sequence number relative to the current `repeat_index`.
        job.seq = self.jobs[index].seq + (self.jobs.len() * self.repeat_index);
        // Inject the job variables, picking the value at the index given by the job sequence
        // number. The values wrap around when the sequence number is greater than the number
        // of values.
        for job_variable in &self.job_variables {
            if job_variable.values.is_empty() {
                continue;
            }
            let value = job_variable.values[job.seq % job_variable.values.len()].clone();
            job.variables.insert(job_variable.name.clone(), value);
        }
        job
    }
}

impl Iterator for JobQueue<'_> {
    type Item = Job;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.jobs.len() {
            self.repeat_index = self.repeat_index.checked_add(1).unwrap_or(0);
            match self.repeat {
                Count::Finite(n) => {
                    if self.repeat_index >= n {
                        None
                    } else {
                        self.index = 1;
                        Some(self.job_at(0))
                    }
                }
                Count::Infinite => {
                    self.index = 1;
                    Some(self.job_at(0))
                }
            }
        } else {
            self.index += 1;
            Some(self.job_at(self.index - 1))
        }
    }
}

#[cfg(test)]
mod tests {
    use hurl_core::input::Input;
    use hurl_core::types::Count;

    use crate::parallel::job::{Job, JobQueue, JobVariable};
    use crate::runner::{RunnerOptionsBuilder, Value, VariableSet};
    use crate::util::logger::LoggerOptionsBuilder;

    fn new_job(file: &str, index: usize) -> Job {
        let variables = VariableSet::new();
        let runner_options = RunnerOptionsBuilder::default().build();
        let logger_options = LoggerOptionsBuilder::default().build();
        Job::new(
            &Input::new(file),
            index,
            &runner_options,
            &variables,
            &logger_options,
        )
    }

    #[test]
    fn job_queue_is_finite() {
        let jobs = [
            new_job("a.hurl", 0),
            new_job("b.hurl", 1),
            new_job("c.hurl", 2),
        ];

        let mut queue = JobQueue::new(&jobs, Count::Finite(2));

        assert_eq!(queue.next(), Some(new_job("a.hurl", 0)));
        assert_eq!(queue.next(), Some(new_job("b.hurl", 1)));
        assert_eq!(queue.next(), Some(new_job("c.hurl", 2)));
        assert_eq!(queue.next(), Some(new_job("a.hurl", 3)));
        assert_eq!(queue.next(), Some(new_job("b.hurl", 4)));
        assert_eq!(queue.next(), Some(new_job("c.hurl", 5)));
        assert_eq!(queue.next(), None);

        assert_eq!(queue.jobs_count(), Count::Finite(6));
    }

    #[test]
    fn input_queue_is_infinite() {
        let jobs = [new_job("foo.hurl", 0)];

        let mut queue = JobQueue::new(&jobs, Count::Infinite);
        assert_eq!(queue.next(), Some(new_job("foo.hurl", 0)));
        assert_eq!(queue.next(), Some(new_job("foo.hurl", 1)));
        assert_eq!(queue.next(), Some(new_job("foo.hurl", 2)));
        assert_eq!(queue.next(), Some(new_job("foo.hurl", 3)));
        assert_eq!(queue.next(), Some(new_job("foo.hurl", 4)));
        // etc...

        assert_eq!(queue.jobs_count(), Count::Infinite);
    }

    #[test]
    fn job_queue_injects_job_variables() {
        let jobs = [
            new_job("a.hurl", 0),
            new_job("b.hurl", 1),
            new_job("c.hurl", 2),
        ];
        let job_variables = vec![JobVariable::new(
            "SUFFIX",
            vec![
                Value::String("red".to_string()),
                Value::String("green".to_string()),
            ],
        )];

        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(1), job_variables);

        let job = queue.next().unwrap();
        assert_eq!(
            job.variables.get("SUFFIX").map(|v| v.value()),
            Some(&Value::String("red".to_string()))
        );
        let job = queue.next().unwrap();
        assert_eq!(
            job.variables.get("SUFFIX").map(|v| v.value()),
            Some(&Value::String("green".to_string()))
        );
        // Values are recycled when there are more jobs than values.
        let job = queue.next().unwrap();
        assert_eq!(
            job.variables.get("SUFFIX").map(|v| v.value()),
            Some(&Value::String("red".to_string()))
        );
        assert_eq!(queue.next(), None);
    }

    #[test]
    fn job_queue_job_variables_with_repeat() {
        let jobs = [new_job("a.hurl", 0), new_job("b.hurl", 1)];
        let job_variables = vec![JobVariable::new(
            "SUFFIX",
            vec![
                Value::String("red".to_string()),
                Value::String("green".to_string()),
                Value::String("blue".to_string()),
            ],
        )];

        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(2), job_variables);

        let suffixes: Vec<String> = (0..4)
            .map(|_| {
                let job = queue.next().unwrap();
                match job.variables.get("SUFFIX").map(|v| v.value()) {
                    Some(Value::String(s)) => s.clone(),
                    other => panic!("unexpected value {other:?}"),
                }
            })
            .collect();
        // Values cycle based on the job sequence number, independently of the repeat round.
        assert_eq!(suffixes, vec!["red", "green", "blue", "red"]);
        assert_eq!(queue.next(), None);
    }

    #[test]
    fn job_queue_job_variables_keep_base_variables() {
        let mut variables = VariableSet::new();
        variables.insert("base".to_string(), Value::String("base".to_string()));
        let runner_options = RunnerOptionsBuilder::default().build();
        let logger_options = LoggerOptionsBuilder::default().build();
        let job = Job::new(
            &Input::new("a.hurl"),
            0,
            &runner_options,
            &variables,
            &logger_options,
        );

        let job_variables = vec![JobVariable::new(
            "SUFFIX",
            vec![Value::String("red".to_string())],
        )];
        let jobs = [job];
        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(1), job_variables);

        let job = queue.next().unwrap();
        assert_eq!(
            job.variables.get("base").map(|v| v.value()),
            Some(&Value::String("base".to_string()))
        );
        assert_eq!(
            job.variables.get("SUFFIX").map(|v| v.value()),
            Some(&Value::String("red".to_string()))
        );
    }

    #[test]
    fn job_queue_job_variables_override_base_variables() {
        let mut variables = VariableSet::new();
        variables.insert("SUFFIX".to_string(), Value::String("base".to_string()));
        let runner_options = RunnerOptionsBuilder::default().build();
        let logger_options = LoggerOptionsBuilder::default().build();
        let job = Job::new(
            &Input::new("a.hurl"),
            0,
            &runner_options,
            &variables,
            &logger_options,
        );

        let job_variables = vec![JobVariable::new(
            "SUFFIX",
            vec![Value::String("red".to_string())],
        )];
        let jobs = [job];
        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(1), job_variables);

        let job = queue.next().unwrap();
        // A job variable shadows a base variable of the same name.
        assert_eq!(
            job.variables.get("SUFFIX").map(|v| v.value()),
            Some(&Value::String("red".to_string()))
        );
    }

    #[test]
    fn job_queue_empty_job_variable_values_are_skipped() {
        let jobs = [new_job("a.hurl", 0)];
        let job_variables = vec![JobVariable::new("SUFFIX", vec![])];

        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(1), job_variables);

        let job = queue.next().unwrap();
        assert!(job.variables.get("SUFFIX").is_none());
    }

    #[test]
    fn job_queue_multiple_job_variables() {
        let jobs = [new_job("a.hurl", 0), new_job("b.hurl", 1)];
        let job_variables = vec![
            JobVariable::new(
                "NAME",
                vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string()),
                    Value::String("c".to_string()),
                ],
            ),
            JobVariable::new(
                "COLOR",
                vec![
                    Value::String("red".to_string()),
                    Value::String("green".to_string()),
                ],
            ),
        ];

        let mut queue = JobQueue::with_job_variables(&jobs, Count::Finite(3), job_variables);

        let expected = [
            ("a", "red"),
            ("b", "green"),
            ("c", "red"),
            ("a", "green"),
            ("b", "red"),
            ("c", "green"),
        ];
        for (expected_name, expected_color) in expected {
            let job = queue.next().unwrap();
            assert_eq!(
                job.variables.get("NAME").map(|v| v.value()),
                Some(&Value::String(expected_name.to_string()))
            );
            assert_eq!(
                job.variables.get("COLOR").map(|v| v.value()),
                Some(&Value::String(expected_color.to_string()))
            );
        }
        assert_eq!(queue.next(), None);
    }
}
